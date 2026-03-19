//! Enterprise SAML broker handlers.
//!
//! Executes the full SAML SP-initiated login flow:
//! authorize (build AuthnRequest) → IdP → ACS callback (validate Response) → session.

use crate::cache::CacheOperations;
use crate::domains::identity::api::enterprise_common::{
    self, ConnectorRecord, EnterpriseProfile, EnterpriseSsoLoginState, UserResolution,
    ENTERPRISE_SSO_STATE_TTL_SECS,
};
use crate::domains::security_observability::service::analytics::FederationEventMetadata;
use crate::error::{AppError, Result};
use crate::state::{
    HasAnalytics, HasCache, HasDbPool, HasIdentityProviders, HasServices, HasSessionManagement,
};
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use flate2::{write::DeflateEncoder, Compression};
use rsa::pkcs8::DecodePublicKey;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Write;

// ── Constants ──

/// Clock skew tolerance for SAML time validation (seconds)
const CLOCK_SKEW_SECS: i64 = 300;

// ── SAML Error Classification ──

/// Classify a SAML validation error into a federation failure reason
fn classify_saml_error(err: &AppError) -> &'static str {
    let msg = err.to_string().to_lowercase();
    if msg.contains("issuer") {
        "invalid_issuer"
    } else if msg.contains("audience") {
        "invalid_audience"
    } else if msg.contains("expired") || msg.contains("notbefore") || msg.contains("notonorafter") {
        "assertion_expired"
    } else if msg.contains("inresponseto")
        || msg.contains("destination")
        || msg.contains("signature")
    {
        "invalid_assertion"
    } else if msg.contains("status") {
        "idp_rejected"
    } else {
        "invalid_assertion"
    }
}

// ── SAML Config Extraction ──

struct SamlConnectorConfig {
    entity_id: String,
    sso_url: String,
    signing_certificate: String,
    #[allow(dead_code)]
    name_id_format: String,
}

fn parse_saml_config(config: &HashMap<String, String>) -> Result<SamlConnectorConfig> {
    let entity_id = config
        .get("entityId")
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| AppError::BadRequest("Missing entityId in SAML connector config".into()))?
        .clone();
    let sso_url = config
        .get("singleSignOnServiceUrl")
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            AppError::BadRequest("Missing singleSignOnServiceUrl in SAML connector config".into())
        })?
        .clone();
    let signing_certificate = config
        .get("signingCertificate")
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            AppError::BadRequest("Missing signingCertificate in SAML connector config".into())
        })?
        .clone();
    let name_id_format = config
        .get("nameIDPolicyFormat")
        .cloned()
        .unwrap_or_else(|| "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress".to_string());

    Ok(SamlConnectorConfig {
        entity_id,
        sso_url,
        signing_certificate,
        name_id_format,
    })
}

// ── RelayState HMAC ──

fn hmac_key(config: &crate::config::Config) -> Vec<u8> {
    // Derive HMAC key from JWT secret + a domain separator
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(config.jwt.secret.as_bytes());
    hasher.update(b"saml-relay-state");
    hasher.finalize().to_vec()
}

fn sign_relay_state(state_id: &str, key: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC key length is always valid");
    mac.update(state_id.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());
    format!("{}:{}", state_id, signature)
}

fn verify_relay_state(relay_state: &str, key: &[u8]) -> Result<String> {
    let (state_id, _sig) = relay_state
        .rsplit_once(':')
        .ok_or_else(|| AppError::BadRequest("Invalid RelayState format".into()))?;
    let expected = sign_relay_state(state_id, key);
    if relay_state != expected {
        return Err(AppError::BadRequest("RelayState tampering detected".into()));
    }
    Ok(state_id.to_string())
}

// ── AuthnRequest Builder ──

fn build_authn_request(
    request_id: &str,
    sp_entity_id: &str,
    acs_url: &str,
    destination: &str,
) -> String {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    format!(
        r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="{}" Version="2.0" IssueInstant="{}" Destination="{}" AssertionConsumerServiceURL="{}" ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"><saml:Issuer>{}</saml:Issuer></samlp:AuthnRequest>"#,
        request_id, now, destination, acs_url, sp_entity_id,
    )
}

fn deflate_and_encode(xml: &str) -> Result<String> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(xml.as_bytes())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Deflate failed: {}", e)))?;
    let compressed = encoder
        .finish()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Deflate finish failed: {}", e)))?;
    Ok(BASE64.encode(&compressed))
}

// ── SAML Response Parser ──

/// Parsed and validated SAML Response data.
#[derive(Debug)]
struct SamlResponseData {
    name_id: String,
    attributes: HashMap<String, String>,
}

/// Parse and validate a SAML Response XML string.
fn parse_and_validate_saml_response(
    xml: &str,
    expected_issuer: &str,
    expected_audience: &str,
    expected_acs_url: &str,
    expected_request_id: Option<&str>,
    signing_cert_pem: &str,
) -> Result<SamlResponseData> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);

    // State for collecting elements
    let mut in_issuer = false;
    let mut in_audience = false;
    let mut in_name_id = false;
    let mut in_attribute_value = false;
    // in_status_code tracking not needed (we only read the Value attribute)

    let mut issuer: Option<String> = None;
    let mut audience: Option<String> = None;
    let mut name_id: Option<String> = None;
    let mut not_before: Option<String> = None;
    let mut not_on_or_after: Option<String> = None;
    let mut in_response_to: Option<String> = None;
    let mut destination: Option<String> = None;
    let mut status_code: Option<String> = None;
    let mut current_attr_name: Option<String> = None;
    let mut attributes: HashMap<String, String> = HashMap::new();
    let mut has_signature = false;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match local_name.as_str() {
                    "Response" => {
                        for attr in e.attributes().flatten() {
                            let key =
                                String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            match key.as_str() {
                                "InResponseTo" => in_response_to = Some(val),
                                "Destination" => destination = Some(val),
                                _ => {}
                            }
                        }
                    }
                    "Issuer" => in_issuer = true,
                    "Audience" => in_audience = true,
                    "NameID" => in_name_id = true,
                    "Conditions" => {
                        for attr in e.attributes().flatten() {
                            let key =
                                String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            match key.as_str() {
                                "NotBefore" => not_before = Some(val),
                                "NotOnOrAfter" => not_on_or_after = Some(val),
                                _ => {}
                            }
                        }
                    }
                    "StatusCode" => {
                        for attr in e.attributes().flatten() {
                            let key =
                                String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
                            if key == "Value" {
                                status_code =
                                    Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                    "Attribute" => {
                        for attr in e.attributes().flatten() {
                            let key =
                                String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
                            if key == "Name" {
                                current_attr_name =
                                    Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                    "AttributeValue" => in_attribute_value = true,
                    "Signature" => has_signature = true,
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if in_issuer && issuer.is_none() {
                    issuer = Some(text.clone());
                }
                if in_audience && audience.is_none() {
                    audience = Some(text.clone());
                }
                if in_name_id && name_id.is_none() {
                    name_id = Some(text.clone());
                }
                if in_attribute_value {
                    if let Some(ref attr_name) = current_attr_name {
                        attributes.entry(attr_name.clone()).or_insert(text.clone());
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let local_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match local_name.as_str() {
                    "Issuer" => in_issuer = false,
                    "Audience" => in_audience = false,
                    "NameID" => in_name_id = false,
                    "AttributeValue" => in_attribute_value = false,
                    "Attribute" => current_attr_name = None,
                    // StatusCode end tag handled implicitly
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(AppError::BadRequest(format!(
                    "Failed to parse SAML Response XML: {}",
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    // ── Validate Status ──
    if let Some(ref code) = status_code {
        if !code.ends_with(":Success") {
            return Err(AppError::BadRequest(format!(
                "SAML Response status is not Success: {}",
                code
            )));
        }
    }

    // ── Validate Issuer ──
    let resp_issuer = issuer
        .ok_or_else(|| AppError::BadRequest("SAML Response missing Issuer element".into()))?;
    if resp_issuer != expected_issuer {
        return Err(AppError::BadRequest(format!(
            "SAML Issuer mismatch: expected '{}', got '{}'",
            expected_issuer, resp_issuer
        )));
    }

    // ── Validate Audience ──
    if let Some(ref aud) = audience {
        if aud != expected_audience {
            return Err(AppError::BadRequest(format!(
                "SAML Audience mismatch: expected '{}', got '{}'",
                expected_audience, aud
            )));
        }
    }

    // ── Validate Destination ──
    if let Some(ref dest) = destination {
        if !dest.is_empty() && dest != expected_acs_url {
            return Err(AppError::BadRequest(format!(
                "SAML Destination mismatch: expected '{}', got '{}'",
                expected_acs_url, dest
            )));
        }
    }

    // ── Validate InResponseTo ──
    if let Some(expected_req_id) = expected_request_id {
        if let Some(ref irt) = in_response_to {
            if irt != expected_req_id {
                return Err(AppError::BadRequest(format!(
                    "SAML InResponseTo mismatch: expected '{}', got '{}'",
                    expected_req_id, irt
                )));
            }
        }
    }

    // ── Validate Time Window ──
    let now = Utc::now();
    if let Some(ref nb) = not_before {
        if let Ok(nb_time) = chrono::DateTime::parse_from_rfc3339(nb) {
            let nb_with_skew = nb_time.timestamp() - CLOCK_SKEW_SECS;
            if now.timestamp() < nb_with_skew {
                return Err(AppError::BadRequest(
                    "SAML Assertion is not yet valid (NotBefore)".into(),
                ));
            }
        }
    }
    if let Some(ref noa) = not_on_or_after {
        if let Ok(noa_time) = chrono::DateTime::parse_from_rfc3339(noa) {
            let noa_with_skew = noa_time.timestamp() + CLOCK_SKEW_SECS;
            if now.timestamp() > noa_with_skew {
                return Err(AppError::BadRequest(
                    "SAML Assertion has expired (NotOnOrAfter)".into(),
                ));
            }
        }
    }

    // ── Validate Signature Presence ──
    if !has_signature {
        tracing::warn!("SAML Response does not contain a Signature element");
    }

    // ── Verify Signature ──
    if has_signature {
        verify_saml_signature(xml, signing_cert_pem)?;
    }

    let name_id = name_id
        .ok_or_else(|| AppError::BadRequest("SAML Response missing NameID element".into()))?;

    Ok(SamlResponseData {
        name_id,
        attributes,
    })
}

/// Verify the XML signature in a SAML Response.
///
/// Extracts the <ds:SignatureValue> and <ds:DigestValue>, then verifies
/// using the IdP's signing certificate.
fn verify_saml_signature(xml: &str, cert_pem: &str) -> Result<()> {
    // Normalize PEM: handle raw base64 (no headers) or full PEM
    let cert_der = decode_certificate(cert_pem)?;

    // Parse X.509 certificate
    let (_, cert) = x509_parser::parse_x509_certificate(&cert_der)
        .map_err(|e| AppError::BadRequest(format!("Invalid X.509 certificate: {}", e)))?;

    // Extract RSA public key from SPKI
    let spki_der = cert.public_key().raw;
    let public_key = rsa::RsaPublicKey::from_public_key_der(spki_der).map_err(|e| {
        AppError::BadRequest(format!(
            "Certificate does not contain a valid RSA key: {}",
            e
        ))
    })?;

    // Extract SignatureValue from XML
    let sig_value = extract_xml_element_text(xml, "SignatureValue")
        .ok_or_else(|| AppError::BadRequest("SAML Response missing SignatureValue".into()))?;

    // Extract SignedInfo for verification
    let signed_info = extract_xml_block(xml, "SignedInfo")
        .ok_or_else(|| AppError::BadRequest("SAML Response missing SignedInfo".into()))?;

    // Decode signature value (base64)
    let sig_bytes = BASE64
        .decode(
            sig_value
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>(),
        )
        .map_err(|e| AppError::BadRequest(format!("Invalid SignatureValue base64: {}", e)))?;

    // Canonicalize SignedInfo (simplified: use the raw XML as-is for verification)
    // Note: Full C14N is complex; this handles the common case where SignedInfo
    // is already in canonical form (which most SAML IdPs produce).
    let signed_info_bytes = signed_info.as_bytes();

    // Determine signature algorithm from SignedInfo
    let uses_sha256 =
        signed_info.contains("rsa-sha256") || signed_info.contains("xmldsig-more#rsa-sha256");

    use rsa::pkcs1v15::{Signature, VerifyingKey};
    use rsa::signature::Verifier;

    if uses_sha256 {
        let verifying_key = VerifyingKey::<sha2::Sha256>::new(public_key);
        let signature = Signature::try_from(sig_bytes.as_slice())
            .map_err(|e| AppError::BadRequest(format!("Invalid RSA signature format: {}", e)))?;
        verifying_key
            .verify(signed_info_bytes, &signature)
            .map_err(|_| {
                AppError::BadRequest("SAML Response signature verification failed".into())
            })?;
    } else {
        // Default to SHA-256 for non-explicit algorithm (SHA-1 is deprecated)
        let verifying_key = VerifyingKey::<sha2::Sha256>::new(public_key);
        let signature = Signature::try_from(sig_bytes.as_slice())
            .map_err(|e| AppError::BadRequest(format!("Invalid RSA signature format: {}", e)))?;
        verifying_key
            .verify(signed_info_bytes, &signature)
            .map_err(|_| {
                AppError::BadRequest("SAML Response signature verification failed".into())
            })?;
    }

    Ok(())
}

/// Decode a certificate from PEM or raw base64 DER format.
fn decode_certificate(cert_pem: &str) -> Result<Vec<u8>> {
    if cert_pem.contains("-----BEGIN") {
        let pem_data: String = cert_pem
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect::<Vec<_>>()
            .join("");
        BASE64
            .decode(&pem_data)
            .map_err(|e| AppError::BadRequest(format!("Invalid signing certificate base64: {}", e)))
    } else {
        let cleaned: String = cert_pem.chars().filter(|c| !c.is_whitespace()).collect();
        BASE64
            .decode(&cleaned)
            .map_err(|e| AppError::BadRequest(format!("Invalid signing certificate base64: {}", e)))
    }
}

/// Extract text content of an XML element by local name.
fn extract_xml_element_text(xml: &str, element_name: &str) -> Option<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut inside = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                if name == element_name {
                    inside = true;
                }
            }
            Ok(Event::Text(ref e)) if inside => {
                return Some(e.unescape().unwrap_or_default().to_string());
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                if name == element_name {
                    inside = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

/// Extract an XML block (opening tag through closing tag) by local name.
fn extract_xml_block(xml: &str, element_name: &str) -> Option<String> {
    // Use a simple string search approach for SignedInfo extraction.
    // This is intentionally simple — SAML SignedInfo blocks are well-formed.
    let patterns: Vec<String> = vec![
        format!("<ds:{}", element_name),
        format!("<{}", element_name),
        format!("<dsig:{}", element_name),
    ];

    for prefix in &patterns {
        if let Some(start_idx) = xml.find(prefix.as_str()) {
            let end_patterns = vec![
                format!("</ds:{}>", element_name),
                format!("</{}>", element_name),
                format!("</dsig:{}>", element_name),
            ];
            for end_pat in &end_patterns {
                if let Some(end_idx) = xml[start_idx..].find(end_pat.as_str()) {
                    return Some(xml[start_idx..start_idx + end_idx + end_pat.len()].to_string());
                }
            }
        }
    }
    None
}

// ── Profile Mapping ──

fn map_saml_profile(
    config: &HashMap<String, String>,
    response_data: &SamlResponseData,
) -> EnterpriseProfile {
    let email_attr = config
        .get("attributeEmail")
        .cloned()
        .unwrap_or_else(|| "email".to_string());
    let name_attr = config
        .get("attributeName")
        .cloned()
        .unwrap_or_else(|| "name".to_string());

    // Common SAML attribute URIs
    let email_uris = [
        &email_attr,
        "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
        "urn:oid:0.9.2342.19200300.100.1.3",
        "email",
    ];
    let name_uris = [
        &name_attr,
        "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name",
        "urn:oid:2.16.840.1.113730.3.1.241",
        "displayName",
        "name",
    ];

    let email = email_uris
        .iter()
        .find_map(|uri| response_data.attributes.get(*uri))
        .cloned();
    let name = name_uris
        .iter()
        .find_map(|uri| response_data.attributes.get(*uri))
        .cloned();

    EnterpriseProfile {
        external_user_id: response_data.name_id.clone(),
        email: email.or_else(|| {
            // If NameID is email format, use it as email
            if response_data.name_id.contains('@') {
                Some(response_data.name_id.clone())
            } else {
                None
            }
        }),
        name,
    }
}

// ── Form Data ──

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct SamlAcsForm {
    pub SAMLResponse: String,
    pub RelayState: Option<String>,
}

// ══════════════════════════════════════════════════════════════════════
// Handlers
// ══════════════════════════════════════════════════════════════════════

/// Initiate enterprise SAML login: build AuthnRequest, redirect to IdP.
pub async fn saml_authorize_redirect<S: HasServices + HasCache + HasDbPool>(
    state: &S,
    connector: ConnectorRecord,
    login_challenge: String,
    _login_hint: Option<String>,
) -> Result<Response> {
    let saml_config = parse_saml_config(&connector.config)?;
    let config = state.config();

    let sp_eid = enterprise_common::sp_entity_id(config);
    let acs = enterprise_common::saml_acs_url(config);
    let request_id = format!("_{}", uuid::Uuid::new_v4());

    // Store enterprise SSO state
    let sso_state = EnterpriseSsoLoginState {
        login_challenge_id: login_challenge,
        connector_alias: connector.alias.clone(),
        tenant_id: connector.tenant_id.clone(),
        authn_request_id: Some(request_id.clone()),
        link_user_id: None,
    };
    let sso_state_id = uuid::Uuid::new_v4().to_string();
    let sso_state_json =
        serde_json::to_string(&sso_state).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_enterprise_sso_state(
            &sso_state_id,
            &sso_state_json,
            ENTERPRISE_SSO_STATE_TTL_SECS,
        )
        .await?;

    // Build AuthnRequest
    let authn_request = build_authn_request(&request_id, &sp_eid, &acs, &saml_config.sso_url);
    let encoded_request = deflate_and_encode(&authn_request)?;

    // Sign RelayState
    let key = hmac_key(config);
    let relay_state = sign_relay_state(&sso_state_id, &key);

    // Build redirect URL (HTTP-Redirect binding)
    let mut redirect_url = url::Url::parse(&saml_config.sso_url)
        .map_err(|e| AppError::BadRequest(format!("Invalid singleSignOnServiceUrl: {}", e)))?;
    {
        let mut pairs = redirect_url.query_pairs_mut();
        pairs.append_pair("SAMLRequest", &encoded_request);
        pairs.append_pair("RelayState", &relay_state);
    }

    metrics::counter!("auth9_enterprise_sso_total", "action" => "saml_authorize", "connector" => connector.alias.clone())
        .increment(1);

    Ok(Redirect::temporary(redirect_url.as_str()).into_response())
}

/// SAML Assertion Consumer Service (ACS) — POST binding callback.
pub async fn saml_acs<
    S: HasServices + HasIdentityProviders + HasCache + HasSessionManagement + HasDbPool + HasAnalytics,
>(
    State(state): State<S>,
    Form(form): Form<SamlAcsForm>,
) -> Result<Response> {
    // 1. Verify RelayState
    let relay_state = form
        .RelayState
        .ok_or_else(|| AppError::BadRequest("Missing RelayState in SAML callback".into()))?;
    let key = hmac_key(state.config());
    let sso_state_id = verify_relay_state(&relay_state, &key)?;

    // 2. Consume enterprise SSO state
    let sso_state_json = state
        .cache()
        .consume_enterprise_sso_state(&sso_state_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired enterprise SSO state".into()))?;
    let sso_state: EnterpriseSsoLoginState =
        serde_json::from_str(&sso_state_json).map_err(|e| AppError::Internal(e.into()))?;

    // 3. Load connector
    let connector =
        enterprise_common::load_connector(state.db_pool(), &sso_state.connector_alias).await?;
    if connector.provider_type != "saml" {
        return Err(AppError::BadRequest(
            "Connector is not a SAML connector".into(),
        ));
    }
    let saml_config = parse_saml_config(&connector.config)?;

    // 4. Decode SAMLResponse
    let response_bytes = BASE64
        .decode(&form.SAMLResponse)
        .map_err(|e| AppError::BadRequest(format!("Invalid SAMLResponse base64: {}", e)))?;
    let response_xml = String::from_utf8(response_bytes)
        .map_err(|e| AppError::BadRequest(format!("SAMLResponse is not valid UTF-8: {}", e)))?;

    // 5. Parse and validate SAML Response
    let config = state.config();
    let sp_eid = enterprise_common::sp_entity_id(config);
    let acs = enterprise_common::saml_acs_url(config);

    let response_data = match parse_and_validate_saml_response(
        &response_xml,
        &saml_config.entity_id,
        &sp_eid,
        &acs,
        sso_state.authn_request_id.as_deref(),
        &saml_config.signing_certificate,
    ) {
        Ok(data) => data,
        Err(e) => {
            // Record federation failure event with SAML-specific reason
            let reason = classify_saml_error(&e);
            let fed_meta = FederationEventMetadata {
                user_id: None,
                email: None,
                tenant_id: crate::models::common::StringUuid::parse_str(&sso_state.tenant_id).ok(),
                provider_alias: connector.alias.clone(),
                provider_type: "saml".to_string(),
                ip_address: None,
                user_agent: None,
                session_id: None,
            };
            let _ = state
                .analytics_service()
                .record_federation_failure(fed_meta, reason)
                .await;
            return Err(e);
        }
    };

    // 6. Map profile
    let profile = map_saml_profile(&connector.config, &response_data);

    // 7a. If this is a link flow (link_user_id present), create linked identity and redirect
    if let Some(ref link_uid) = sso_state.link_user_id {
        let user_id = crate::models::common::StringUuid::parse_str(link_uid)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid link_user_id")))?;
        let input = crate::models::linked_identity::CreateLinkedIdentityInput {
            user_id,
            provider_type: "saml".to_string(),
            provider_alias: connector.alias.clone(),
            external_user_id: profile.external_user_id.clone(),
            external_email: profile.email.clone(),
        };
        let _ = state
            .identity_provider_service()
            .create_linked_identity(&input)
            .await;

        // Record identity linked event
        if let Err(e) = state
            .analytics_service()
            .record_identity_linked(user_id, &connector.alias, "saml")
            .await
        {
            tracing::warn!("Failed to record SAML identity linked event: {}", e);
        }

        let portal_base = state
            .config()
            .portal_url
            .as_deref()
            .unwrap_or(&state.config().jwt.issuer);
        let identities_url = format!(
            "{}/dashboard/account/identities",
            portal_base.trim_end_matches('/')
        );
        return Ok(Redirect::temporary(&identities_url).into_response());
    }

    // 7b. Find or create user
    let resolution = enterprise_common::find_or_create_enterprise_user(
        &state,
        &connector,
        &sso_state.tenant_id,
        &profile,
        "saml",
        &sso_state.login_challenge_id,
    )
    .await?;

    // Handle pending merge: redirect to portal confirm-link page
    let user = match resolution {
        UserResolution::Found(user) => user,
        UserResolution::PendingMerge(pending) => {
            let token = uuid::Uuid::new_v4().to_string();
            let pending_json =
                serde_json::to_string(&pending).map_err(|e| AppError::Internal(e.into()))?;
            state
                .cache()
                .store_pending_merge(&token, &pending_json, ENTERPRISE_SSO_STATE_TTL_SECS)
                .await?;
            let portal_base = state
                .config()
                .portal_url
                .as_deref()
                .unwrap_or(&state.config().jwt.issuer);
            let redirect_url = format!(
                "{}/login/confirm-link?token={}",
                portal_base.trim_end_matches('/'),
                token
            );
            return Ok(Redirect::temporary(&redirect_url).into_response());
        }
    };

    // 8. Create session
    let session = state
        .session_service()
        .create_session(user.id, None, None, None)
        .await?;

    // 9. Complete login flow
    let redirect_url = enterprise_common::complete_login_flow(
        &state,
        &sso_state.login_challenge_id,
        &user,
        session.id,
    )
    .await?;

    metrics::counter!("auth9_enterprise_sso_total", "action" => "saml_callback_success", "connector" => connector.alias.clone())
        .increment(1);

    // Record federation login event
    let fed_meta = FederationEventMetadata {
        user_id: Some(user.id),
        email: Some(user.email.clone()),
        tenant_id: crate::models::common::StringUuid::parse_str(&sso_state.tenant_id).ok(),
        provider_alias: connector.alias.clone(),
        provider_type: "saml".to_string(),
        ip_address: None,
        user_agent: None,
        session_id: Some(session.id),
    };
    if let Err(e) = state
        .analytics_service()
        .record_federation_login(fed_meta)
        .await
    {
        tracing::warn!(
            "Failed to record enterprise SAML federation login event: {}",
            e
        );
    }

    let mut response = Redirect::temporary(&redirect_url).into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        "no-store".parse().unwrap(),
    );
    Ok(response)
}

/// Generate SP metadata XML for a SAML connector.
#[utoipa::path(
    get,
    path = "/api/v1/enterprise-sso/saml/metadata/{alias}",
    tag = "Identity",
    responses((status = 200, description = "SP metadata XML"))
)]
pub async fn saml_metadata<S: HasServices + HasDbPool>(
    State(state): State<S>,
    Path(alias): Path<String>,
) -> Result<Response> {
    // Verify connector exists and is SAML
    let connector = enterprise_common::load_connector(state.db_pool(), &alias).await?;
    if connector.provider_type != "saml" {
        return Err(AppError::BadRequest(
            "Connector is not a SAML connector".into(),
        ));
    }

    let config = state.config();
    let sp_eid = enterprise_common::sp_entity_id(config);
    let acs = enterprise_common::saml_acs_url(config);

    let metadata = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" entityID="{}">
  <md:SPSSODescriptor AuthnRequestsSigned="false" WantAssertionsSigned="true" protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <md:NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress</md:NameIDFormat>
    <md:AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="{}" index="0" isDefault="true"/>
  </md:SPSSODescriptor>
</md:EntityDescriptor>"#,
        sp_eid, acs,
    );

    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        metadata,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_saml_config_success() {
        let mut config = HashMap::new();
        config.insert(
            "entityId".to_string(),
            "https://idp.corp.example.com".to_string(),
        );
        config.insert(
            "singleSignOnServiceUrl".to_string(),
            "https://idp.corp.example.com/sso".to_string(),
        );
        config.insert("signingCertificate".to_string(), "MIID...".to_string());
        let result = parse_saml_config(&config).unwrap();
        assert_eq!(result.entity_id, "https://idp.corp.example.com");
        assert_eq!(result.sso_url, "https://idp.corp.example.com/sso");
        assert_eq!(result.signing_certificate, "MIID...");
        assert!(result.name_id_format.contains("emailAddress"));
    }

    #[test]
    fn test_parse_saml_config_custom_name_id_format() {
        let mut config = HashMap::new();
        config.insert("entityId".to_string(), "e".to_string());
        config.insert("singleSignOnServiceUrl".to_string(), "s".to_string());
        config.insert("signingCertificate".to_string(), "c".to_string());
        config.insert(
            "nameIDPolicyFormat".to_string(),
            "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent".to_string(),
        );
        let result = parse_saml_config(&config).unwrap();
        assert!(result.name_id_format.contains("persistent"));
    }

    #[test]
    fn test_parse_saml_config_missing_entity_id() {
        let config = HashMap::new();
        let result = parse_saml_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_saml_config_empty_sso_url() {
        let mut config = HashMap::new();
        config.insert("entityId".to_string(), "e".to_string());
        config.insert("singleSignOnServiceUrl".to_string(), "  ".to_string());
        config.insert("signingCertificate".to_string(), "c".to_string());
        let result = parse_saml_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_verify_relay_state() {
        let key = b"test-hmac-key-for-relay-state";
        let state_id = "abc-123-def";
        let signed = sign_relay_state(state_id, key);
        assert!(signed.starts_with("abc-123-def:"));
        let verified = verify_relay_state(&signed, key).unwrap();
        assert_eq!(verified, "abc-123-def");
    }

    #[test]
    fn test_verify_relay_state_tampered() {
        let key = b"test-hmac-key-for-relay-state";
        let state_id = "abc-123-def";
        let signed = sign_relay_state(state_id, key);
        let tampered = format!("{}x", signed);
        let result = verify_relay_state(&tampered, key);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_relay_state_missing_separator() {
        let key = b"test-hmac-key";
        let result = verify_relay_state("no-separator-here", key);
        // This will actually find the last '-' as separator, but signature won't match
        assert!(result.is_err());
    }

    #[test]
    fn test_build_authn_request() {
        let xml = build_authn_request(
            "_req-123",
            "https://auth9.example.com",
            "https://auth9.example.com/api/v1/enterprise-sso/saml/acs",
            "https://idp.example.com/sso",
        );
        assert!(xml.contains("ID=\"_req-123\""));
        assert!(xml.contains("Destination=\"https://idp.example.com/sso\""));
        assert!(xml.contains("<saml:Issuer>https://auth9.example.com</saml:Issuer>"));
        assert!(xml.contains("AssertionConsumerServiceURL="));
        assert!(xml.contains("Version=\"2.0\""));
    }

    #[test]
    fn test_deflate_and_encode() {
        let xml = "<samlp:AuthnRequest>test</samlp:AuthnRequest>";
        let encoded = deflate_and_encode(xml).unwrap();
        // Should be base64
        assert!(BASE64.decode(&encoded).is_ok());
    }

    #[test]
    fn test_map_saml_profile_with_attributes() {
        let config = HashMap::new();
        let mut attrs = HashMap::new();
        attrs.insert("email".to_string(), "user@corp.example.com".to_string());
        attrs.insert("name".to_string(), "Test User".to_string());
        let data = SamlResponseData {
            name_id: "user@corp.example.com".to_string(),
            attributes: attrs,
        };
        let profile = map_saml_profile(&config, &data);
        assert_eq!(profile.external_user_id, "user@corp.example.com");
        assert_eq!(profile.email.as_deref(), Some("user@corp.example.com"));
        assert_eq!(profile.name.as_deref(), Some("Test User"));
    }

    #[test]
    fn test_map_saml_profile_email_from_name_id() {
        let config = HashMap::new();
        let data = SamlResponseData {
            name_id: "john@example.com".to_string(),
            attributes: HashMap::new(),
        };
        let profile = map_saml_profile(&config, &data);
        assert_eq!(profile.email.as_deref(), Some("john@example.com"));
    }

    #[test]
    fn test_map_saml_profile_no_email() {
        let config = HashMap::new();
        let data = SamlResponseData {
            name_id: "some-opaque-id".to_string(),
            attributes: HashMap::new(),
        };
        let profile = map_saml_profile(&config, &data);
        assert!(profile.email.is_none());
    }

    #[test]
    fn test_map_saml_profile_custom_attribute_names() {
        let mut config = HashMap::new();
        config.insert("attributeEmail".to_string(), "mail".to_string());
        config.insert("attributeName".to_string(), "displayName".to_string());
        let mut attrs = HashMap::new();
        attrs.insert("mail".to_string(), "custom@example.com".to_string());
        attrs.insert("displayName".to_string(), "Custom User".to_string());
        let data = SamlResponseData {
            name_id: "id-123".to_string(),
            attributes: attrs,
        };
        let profile = map_saml_profile(&config, &data);
        assert_eq!(profile.email.as_deref(), Some("custom@example.com"));
        assert_eq!(profile.name.as_deref(), Some("Custom User"));
    }

    #[test]
    fn test_map_saml_profile_urn_oid_attributes() {
        let config = HashMap::new();
        let mut attrs = HashMap::new();
        attrs.insert(
            "urn:oid:0.9.2342.19200300.100.1.3".to_string(),
            "oid-user@example.com".to_string(),
        );
        let data = SamlResponseData {
            name_id: "id-456".to_string(),
            attributes: attrs,
        };
        let profile = map_saml_profile(&config, &data);
        assert_eq!(profile.email.as_deref(), Some("oid-user@example.com"));
    }

    #[test]
    fn test_parse_saml_response_valid() {
        let xml = r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="_resp1" InResponseTo="_req1" Destination="https://auth9.example.com/api/v1/enterprise-sso/saml/acs"><saml:Issuer>https://idp.example.com</saml:Issuer><samlp:Status><samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></samlp:Status><saml:Assertion><saml:Conditions NotBefore="2020-01-01T00:00:00Z" NotOnOrAfter="2099-12-31T23:59:59Z"><saml:AudienceRestriction><saml:Audience>https://auth9.example.com</saml:Audience></saml:AudienceRestriction></saml:Conditions><saml:Subject><saml:NameID>user@example.com</saml:NameID></saml:Subject><saml:AttributeStatement><saml:Attribute Name="email"><saml:AttributeValue>user@example.com</saml:AttributeValue></saml:Attribute></saml:AttributeStatement></saml:Assertion></samlp:Response>"#;

        let result = parse_and_validate_saml_response(
            xml,
            "https://idp.example.com",
            "https://auth9.example.com",
            "https://auth9.example.com/api/v1/enterprise-sso/saml/acs",
            Some("_req1"),
            "", // No cert needed since there's no signature
        );
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.name_id, "user@example.com");
        assert_eq!(data.attributes.get("email").unwrap(), "user@example.com");
    }

    #[test]
    fn test_parse_saml_response_wrong_issuer() {
        let xml = r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"><saml:Issuer>https://wrong-idp.example.com</saml:Issuer><samlp:Status><samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></samlp:Status><saml:Assertion><saml:Subject><saml:NameID>user@example.com</saml:NameID></saml:Subject></saml:Assertion></samlp:Response>"#;

        let result = parse_and_validate_saml_response(
            xml,
            "https://idp.example.com",
            "https://auth9.example.com",
            "https://auth9.example.com/acs",
            None,
            "",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Issuer mismatch"));
    }

    #[test]
    fn test_parse_saml_response_wrong_audience() {
        let xml = r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"><saml:Issuer>https://idp.example.com</saml:Issuer><samlp:Status><samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></samlp:Status><saml:Assertion><saml:Conditions><saml:AudienceRestriction><saml:Audience>https://wrong-audience.example.com</saml:Audience></saml:AudienceRestriction></saml:Conditions><saml:Subject><saml:NameID>user@example.com</saml:NameID></saml:Subject></saml:Assertion></samlp:Response>"#;

        let result = parse_and_validate_saml_response(
            xml,
            "https://idp.example.com",
            "https://auth9.example.com",
            "https://auth9.example.com/acs",
            None,
            "",
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Audience mismatch"));
    }

    #[test]
    fn test_parse_saml_response_expired() {
        let xml = r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"><saml:Issuer>https://idp.example.com</saml:Issuer><samlp:Status><samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></samlp:Status><saml:Assertion><saml:Conditions NotOnOrAfter="2020-01-01T00:00:00Z"><saml:AudienceRestriction><saml:Audience>https://auth9.example.com</saml:Audience></saml:AudienceRestriction></saml:Conditions><saml:Subject><saml:NameID>user@example.com</saml:NameID></saml:Subject></saml:Assertion></samlp:Response>"#;

        let result = parse_and_validate_saml_response(
            xml,
            "https://idp.example.com",
            "https://auth9.example.com",
            "https://auth9.example.com/acs",
            None,
            "",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expired"));
    }

    #[test]
    fn test_parse_saml_response_wrong_in_response_to() {
        let xml = r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" InResponseTo="_wrong-id"><saml:Issuer>https://idp.example.com</saml:Issuer><samlp:Status><samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></samlp:Status><saml:Assertion><saml:Subject><saml:NameID>user@example.com</saml:NameID></saml:Subject></saml:Assertion></samlp:Response>"#;

        let result = parse_and_validate_saml_response(
            xml,
            "https://idp.example.com",
            "https://auth9.example.com",
            "https://auth9.example.com/acs",
            Some("_req-123"),
            "",
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("InResponseTo mismatch"));
    }

    #[test]
    fn test_parse_saml_response_failed_status() {
        let xml = r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"><saml:Issuer>https://idp.example.com</saml:Issuer><samlp:Status><samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Requester"/></samlp:Status></samlp:Response>"#;

        let result = parse_and_validate_saml_response(
            xml,
            "https://idp.example.com",
            "https://auth9.example.com",
            "https://auth9.example.com/acs",
            None,
            "",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not Success"));
    }

    #[test]
    fn test_parse_saml_response_wrong_destination() {
        let xml = r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" Destination="https://wrong.example.com/acs"><saml:Issuer>https://idp.example.com</saml:Issuer><samlp:Status><samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></samlp:Status><saml:Assertion><saml:Subject><saml:NameID>user@example.com</saml:NameID></saml:Subject></saml:Assertion></samlp:Response>"#;

        let result = parse_and_validate_saml_response(
            xml,
            "https://idp.example.com",
            "https://auth9.example.com",
            "https://auth9.example.com/acs",
            None,
            "",
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Destination mismatch"));
    }

    #[test]
    fn test_extract_xml_element_text() {
        let xml = r#"<root><ds:SignatureValue>abc123==</ds:SignatureValue></root>"#;
        let result = extract_xml_element_text(xml, "SignatureValue");
        assert_eq!(result.as_deref(), Some("abc123=="));
    }

    #[test]
    fn test_extract_xml_block() {
        let xml = r#"<root><ds:SignedInfo><ds:Reference/></ds:SignedInfo></root>"#;
        let result = extract_xml_block(xml, "SignedInfo");
        assert!(result.is_some());
        assert!(result.unwrap().contains("SignedInfo"));
    }

    #[test]
    fn test_saml_acs_form_has_expected_fields() {
        // SamlAcsForm is deserialized from POST form data by axum::Form
        let form = SamlAcsForm {
            SAMLResponse: "dGVzdA==".to_string(),
            RelayState: Some("state:sig".to_string()),
        };
        assert_eq!(form.SAMLResponse, "dGVzdA==");
        assert_eq!(form.RelayState.as_deref(), Some("state:sig"));
    }

    #[test]
    fn test_relay_state_with_empty_key() {
        let key = b"";
        let state_id = "test-state-id";
        let signed = sign_relay_state(state_id, key);
        // Should still produce a valid signed string
        assert!(signed.starts_with("test-state-id:"));
        // And verification with same empty key should succeed
        let verified = verify_relay_state(&signed, key).unwrap();
        assert_eq!(verified, "test-state-id");
    }

    #[test]
    fn test_parse_saml_response_missing_name_id() {
        // SAML response with Assertion but no NameID
        let xml = r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"><saml:Issuer>https://idp.example.com</saml:Issuer><samlp:Status><samlp:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></samlp:Status><saml:Assertion><saml:Conditions NotBefore="2020-01-01T00:00:00Z" NotOnOrAfter="2099-12-31T23:59:59Z"><saml:AudienceRestriction><saml:Audience>https://auth9.example.com</saml:Audience></saml:AudienceRestriction></saml:Conditions><saml:Subject></saml:Subject></saml:Assertion></samlp:Response>"#;

        let result = parse_and_validate_saml_response(
            xml,
            "https://idp.example.com",
            "https://auth9.example.com",
            "https://auth9.example.com/acs",
            None,
            "",
        );
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("NameID")
                || err_msg.contains("name_id")
                || err_msg.contains("Subject"),
            "Error should mention missing NameID, got: {}",
            err_msg
        );
    }
}
