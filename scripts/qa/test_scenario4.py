from playwright.sync_api import sync_playwright
import sys
import json
import base64

def decode_jwt_payload(token):
    """Decode JWT payload (second part)"""
    parts = token.split('.')
    if len(parts) >= 2:
        payload = parts[1]
        # Add padding if needed
        padding = 4 - len(payload) % 4
        if padding != 4:
            payload += '=' * padding
        decoded = base64.urlsafe_b64decode(payload)
        return json.loads(decoded)
    return None

def run():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context()
        page = context.new_page()
        
        try:
            # Step 1: Login first
            print('Step 1: Navigate to demo login...')
            page.goto('http://localhost:3002/login')
            page.wait_for_load_state('networkidle')
            
            print('Step 2: Waiting for Keycloak login page...')
            page.wait_for_selector('input[name="username"]', timeout=10000)
            
            print('Step 3: Filling credentials...')
            page.fill('input[name="username"]', 'admin')
            page.fill('input[name="password"]', 'NewSecurePass456!')
            page.click('button[type="submit"]')
            
            print('Step 4: Waiting for dashboard...')
            page.wait_for_url('**/dashboard**', timeout=15000)
            
            # Step 2: Find and click the exchange token button
            print('Step 5: Looking for exchange token button...')
            
            # Wait for the button to be visible
            page.wait_for_selector('button:has-text("Exchange Token")', timeout=5000)
            
            # Click the button
            page.click('button:has-text("Exchange Token")')
            
            # Wait for response
            page.wait_for_timeout(2000)
            
            # Get page content
            page_content = page.content()
            
            # Check for token response
            has_access_token = 'accessToken' in page_content
            has_token_type = 'tokenType' in page_content
            has_expires_in = 'expiresIn' in page_content
            has_tenant_id = 'tenant_id' in page_content
            has_roles = 'roles' in page_content
            
            print(f'Has accessToken: {has_access_token}')
            print(f'Has tokenType: {has_token_type}')
            print(f'Has expiresIn: {has_expires_in}')
            print(f'Has tenant_id: {has_tenant_id}')
            print(f'Has roles: {has_roles}')
            
            # Check for errors
            has_errors = 'InvalidSignature' in page_content or 'Invalid tenant ID' in page_content or 'Client not found' in page_content
            print(f'Has errors: {has_errors}')
            
            print('\n=== Scenario 4 Result ===')
            if has_access_token and not has_errors:
                print('PASS: Token exchange successful')
            else:
                print('FAIL: Token exchange failed')
                sys.exit(1)
            
        except Exception as e:
            print('ERROR:', str(e))
            print('\n=== Scenario 4 Result ===')
            print('FAIL:', str(e))
            sys.exit(1)
        finally:
            browser.close()

if __name__ == '__main__':
    run()
