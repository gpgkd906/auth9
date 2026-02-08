import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  tenantApi,
  userApi,
  serviceApi,
  rbacApi,
  auditApi,
  systemApi,
  invitationApi,
  brandingApi,
  emailTemplateApi,
  analyticsApi,
  webhookApi,
  tenantServiceApi,
  securityAlertApi,
  passwordApi,
  sessionApi,
  webauthnApi,
  identityProviderApi,
  publicBrandingApi,
  type Tenant,
  type User,
  type Service,
  type PaginatedResponse,
} from '~/services/api';

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('API Service', () => {
  beforeEach(() => {
    mockFetch.mockClear();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('handleResponse', () => {
    it('should throw error on non-ok response', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        json: async () => ({ error: 'bad_request', message: 'Invalid input' }),
      });

      await expect(tenantApi.list()).rejects.toThrow('Invalid input');
    });

    it('should return data on successful response', async () => {
      const mockData: PaginatedResponse<Tenant> = {
        data: [{ id: '1', name: 'Test', slug: 'test', settings: {}, status: 'active', created_at: '', updated_at: '' }],
        pagination: { page: 1, per_page: 20, total: 1, total_pages: 1 },
      };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => mockData,
      });

      const result = await tenantApi.list();
      expect(result).toEqual(mockData);
    });
  });

  describe('tenantApi', () => {
    it('should list tenants with default pagination', async () => {
      const mockData: PaginatedResponse<Tenant> = {
        data: [],
        pagination: { page: 1, per_page: 20, total: 0, total_pages: 0 },
      };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => mockData,
      });

      await tenantApi.list();

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants?page=1&per_page=20'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should list tenants with custom pagination', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [], pagination: { page: 2, per_page: 50, total: 0, total_pages: 0 } }),
      });

      await tenantApi.list(2, 50);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants?page=2&per_page=50'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should get a tenant by ID', async () => {
      const mockTenant: Tenant = {
        id: '123',
        name: 'Test Tenant',
        slug: 'test-tenant',
        settings: {},
        status: 'active',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: mockTenant }),
      });

      const result = await tenantApi.get('123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/123'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
      expect(result.data).toEqual(mockTenant);
    });

    it('should create a tenant', async () => {
      const input = { name: 'New Tenant', slug: 'new-tenant' };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { id: '456', ...input, settings: {}, status: 'active', created_at: '', updated_at: '' } }),
      });

      await tenantApi.create(input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants'),
        expect.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(input),
        })
      );
    });

    it('should update a tenant', async () => {
      const input = { name: 'Updated Name' };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { id: '123', name: 'Updated Name', slug: 'test', settings: {}, status: 'active', created_at: '', updated_at: '' } }),
      });

      await tenantApi.update('123', input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/123'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify(input),
        })
      );
    });

    it('should delete a tenant', async () => {
      mockFetch.mockResolvedValue({ ok: true });

      await tenantApi.delete('123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/123'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });
  });

  describe('userApi', () => {
    it('should list users', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [], pagination: { page: 1, per_page: 20, total: 0, total_pages: 0 } }),
      });

      await userApi.list();

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users?page=1&per_page=20'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should get a user by ID', async () => {
      const mockUser: User = {
        id: '123',
        email: 'test@example.com',
        display_name: 'Test User',
        mfa_enabled: false,
        created_at: '',
        updated_at: '',
      };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: mockUser }),
      });

      const result = await userApi.get('123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/123')
      );
      expect(result.data).toEqual(mockUser);
    });

    it('should create a user with password', async () => {
      const input = { email: 'new@example.com', display_name: 'New User', password: 'secret123' };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { id: '456', email: input.email, display_name: input.display_name, mfa_enabled: false, created_at: '', updated_at: '' } }),
      });

      await userApi.create(input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ email: input.email, display_name: input.display_name, password: input.password }),
        })
      );
    });

    it('should update a user', async () => {
      const input = { display_name: 'Updated Name' };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { id: '123', email: 'test@example.com', display_name: 'Updated Name', mfa_enabled: false, created_at: '', updated_at: '' } }),
      });

      await userApi.update('123', input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/123'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify(input),
        })
      );
    });

    it('should get user tenants', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [] }),
      });

      await userApi.getTenants('123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/123/tenants'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should add user to tenant', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({}),
      });

      await userApi.addToTenant('user-123', 'tenant-456', 'admin');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/user-123/tenants'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ tenant_id: 'tenant-456', role_in_tenant: 'admin' }),
        })
      );
    });

    it('should remove user from tenant', async () => {
      mockFetch.mockResolvedValue({ ok: true });

      await userApi.removeFromTenant('user-123', 'tenant-456');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/user-123/tenants/tenant-456'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });
  });

  describe('serviceApi', () => {
    it('should list services', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [], pagination: { page: 1, per_page: 20, total: 0, total_pages: 0 } }),
      });

      // serviceApi.list(tenantId?, page, perPage, accessToken?)
      await serviceApi.list(undefined, 1, 20);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/services?page=1&per_page=20'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should list services filtered by tenant', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [], pagination: { page: 1, per_page: 20, total: 0, total_pages: 0 } }),
      });

      // serviceApi.list(tenantId?, page, perPage, accessToken?)
      await serviceApi.list('tenant-123', 1, 20);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('tenant_id=tenant-123'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should get a service by ID', async () => {
      const mockService: Service = {
        id: '123',
        name: 'Test Service',
        redirect_uris: [],
        logout_uris: [],
        status: 'active',
        created_at: '',
        updated_at: '',
      };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: mockService }),
      });

      const result = await serviceApi.get('123');
      expect(result.data).toEqual(mockService);
    });

    it('should create a service', async () => {
      const input = { name: 'New Service', client_id: 'new-service', redirect_uris: ['http://localhost'] };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: {
            service: { id: '456', ...input, logout_uris: [], status: 'active', created_at: '', updated_at: '' },
            client: { client: { id: '789', service_id: '456', client_id: 'new-service', created_at: '' }, client_secret: 'secret' },
          },
        }),
      });

      await serviceApi.create(input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/services'),
        expect.objectContaining({ method: 'POST' })
      );
    });

    it('should delete a service', async () => {
      mockFetch.mockResolvedValue({ ok: true });

      await serviceApi.delete('123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/services/123'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });

    it('should list clients for a service', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [] }),
      });

      await serviceApi.listClients('service-123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/services/service-123/clients'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should create a client for a service', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: {
            client: { id: '123', service_id: 'svc', client_id: 'new-client', created_at: '' },
            client_secret: 'secret123',
          },
        }),
      });

      await serviceApi.createClient('service-123', { name: 'New Client' });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/services/service-123/clients'),
        expect.objectContaining({ method: 'POST' })
      );
    });

    it('should delete a client', async () => {
      mockFetch.mockResolvedValue({ ok: true });

      await serviceApi.deleteClient('service-123', 'client-456');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/services/service-123/clients/client-456'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });

    it('should regenerate client secret', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { client_id: 'client-456', client_secret: 'new-secret' } }),
      });

      const result = await serviceApi.regenerateClientSecret('service-123', 'client-456');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/services/service-123/clients/client-456/regenerate-secret'),
        expect.objectContaining({ method: 'POST' })
      );
      expect(result.data.client_secret).toBe('new-secret');
    });
  });

  describe('rbacApi', () => {
    it('should list roles for a service', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [] }),
      });

      await rbacApi.listRoles('service-123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/services/service-123/roles'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should create a role', async () => {
      const input = { name: 'admin', description: 'Administrator' };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { id: '456', service_id: 'service-123', ...input, created_at: '', updated_at: '' } }),
      });

      await rbacApi.createRole('service-123', input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/roles'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ ...input, service_id: 'service-123' }),
        })
      );
    });

    it('should update a role', async () => {
      const input = { name: 'super-admin' };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { id: '456', service_id: 'service-123', name: 'super-admin', created_at: '', updated_at: '' } }),
      });

      await rbacApi.updateRole('service-123', 'role-456', input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/roles/role-456'),
        expect.objectContaining({ method: 'PUT' })
      );
    });

    it('should delete a role', async () => {
      mockFetch.mockResolvedValue({ ok: true });

      await rbacApi.deleteRole('service-123', 'role-456');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/roles/role-456'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });

    it('should list permissions for a service', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [] }),
      });

      await rbacApi.listPermissions('service-123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/services/service-123/permissions'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should assign roles to user', async () => {
      const input = { user_id: 'user-123', tenant_id: 'tenant-456', roles: ['role-1', 'role-2'] };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({}),
      });

      await rbacApi.assignRoles(input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/rbac/assign'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify(input),
        })
      );
    });

    it('should get user roles in tenant', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: { user_id: 'user-123', tenant_id: 'tenant-456', roles: ['admin'], permissions: ['read', 'write'] },
        }),
      });

      const result = await rbacApi.getUserRoles('user-123', 'tenant-456');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/user-123/tenants/tenant-456/roles')
      );
      expect(result.data.roles).toContain('admin');
    });

    it('should unassign role from user', async () => {
      mockFetch.mockResolvedValue({ ok: true });

      await rbacApi.unassignRole('user-123', 'tenant-456', 'role-789');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/user-123/tenants/tenant-456/roles/role-789'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });
  });

  describe('auditApi', () => {
    it('should list audit logs with pagination', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: [],
          pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
        }),
      });

      await auditApi.list(1, 50);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/audit-logs?limit=50&offset=0'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should calculate correct offset for pagination', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: [],
          pagination: { page: 3, per_page: 50, total: 0, total_pages: 0 },
        }),
      });

      await auditApi.list(3, 50);

      // Page 3 with 50 per page = offset of 100
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/audit-logs?limit=50&offset=100'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });
  });

  describe('Error handling', () => {
    it('should handle network errors gracefully', async () => {
      mockFetch.mockRejectedValue(new Error('Network error'));

      await expect(tenantApi.list()).rejects.toThrow('Network error');
    });

    it('should handle non-JSON error responses', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        statusText: 'Internal Server Error',
        json: async () => { throw new Error('Not JSON'); },
      });

      await expect(tenantApi.list()).rejects.toThrow('Internal Server Error');
    });
  });

  describe('systemApi', () => {
    it('should get email settings', async () => {
      const mockSettings = {
        data: {
          category: 'email',
          setting_key: 'provider',
          value: { type: 'smtp', host: 'smtp.test.com', port: 587 },
          updated_at: new Date().toISOString(),
        },
      };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => mockSettings,
      });

      const result = await systemApi.getEmailSettings();
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/email'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
      expect(result.data.value.type).toBe('smtp');
    });

    it('should update email settings', async () => {
      const config = { type: 'smtp' as const, host: 'smtp.new.com', port: 587, use_tls: true, from_email: 'test@test.com' };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { category: 'email', value: config } }),
      });

      await systemApi.updateEmailSettings(config);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/email'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({ config }),
        })
      );
    });

    it('should test email connection', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ success: true, message: 'Connection successful' }),
      });

      const result = await systemApi.testEmailConnection();

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/email/test'),
        expect.objectContaining({ method: 'POST' })
      );
      expect(result.success).toBe(true);
    });

    it('should send test email', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ success: true, message: 'Email sent', message_id: 'msg-123' }),
      });

      const result = await systemApi.sendTestEmail('test@example.com');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/email/send-test'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ to_email: 'test@example.com' }),
        })
      );
      expect(result.success).toBe(true);
    });
  });

  describe('invitationApi', () => {
    it('should list invitations for a tenant', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [], pagination: { page: 1, per_page: 20, total: 0, total_pages: 0 } }),
      });

      await invitationApi.list('tenant-123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-123/invitations?page=1&per_page=20'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should create an invitation', async () => {
      const input = { email: 'new@example.com', role_ids: ['role-1'], expires_in_hours: 48 };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: { id: 'inv-123', tenant_id: 'tenant-123', ...input, status: 'pending', invited_by: 'admin', expires_at: '', created_at: '' },
        }),
      });

      await invitationApi.create('tenant-123', input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-123/invitations'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify(input),
        })
      );
    });

    it('should get an invitation by ID', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: { id: 'inv-123', tenant_id: 'tenant-123', email: 'test@test.com', status: 'pending' },
        }),
      });

      const result = await invitationApi.get('inv-123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/invitations/inv-123'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
      expect(result.data.id).toBe('inv-123');
    });

    it('should revoke an invitation', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: { id: 'inv-123', status: 'revoked' },
        }),
      });

      const result = await invitationApi.revoke('inv-123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/invitations/inv-123/revoke'),
        expect.objectContaining({ method: 'POST' })
      );
      expect(result.data.status).toBe('revoked');
    });

    it('should resend an invitation', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: { id: 'inv-123', status: 'pending' },
        }),
      });

      await invitationApi.resend('inv-123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/invitations/inv-123/resend'),
        expect.objectContaining({ method: 'POST' })
      );
    });

    it('should delete an invitation', async () => {
      mockFetch.mockResolvedValue({ ok: true });

      await invitationApi.delete('inv-123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/invitations/inv-123'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });

    it('should throw error when deleting invitation fails', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        json: async () => ({ error: 'not_found', message: 'Invitation not found' }),
      });

      await expect(invitationApi.delete('inv-123')).rejects.toThrow('Invitation not found');
    });
  });

  describe('brandingApi', () => {
    it('should get branding config', async () => {
      const mockConfig = {
        data: {
          primary_color: '#007AFF',
          secondary_color: '#5856D6',
          background_color: '#F5F5F7',
          text_color: '#1D1D1F',
          allow_registration: false,
        },
      };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => mockConfig,
      });

      const result = await brandingApi.get();

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/branding'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
      expect(result.data.primary_color).toBe('#007AFF');
    });

    it('should update branding config', async () => {
      const config = {
        primary_color: '#FF0000',
        secondary_color: '#00FF00',
        background_color: '#0000FF',
        text_color: '#FFFFFF',
        allow_registration: true,
      };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: config }),
      });

      await brandingApi.update(config);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/branding'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({ config }),
        })
      );
    });
  });

  describe('emailTemplateApi', () => {
    it('should list email templates', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [] }),
      });

      await emailTemplateApi.list();

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/email-templates'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should get a specific email template', async () => {
      const mockTemplate = {
        metadata: { template_type: 'invitation', name: 'Invitation', description: '', variables: [] },
        content: { subject: 'You are invited', html_body: '<p>Hello</p>', text_body: 'Hello' },
        is_customized: false,
      };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: mockTemplate }),
      });

      const result = await emailTemplateApi.get('invitation');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/email-templates/invitation'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
      expect(result.data.metadata.template_type).toBe('invitation');
    });

    it('should update an email template', async () => {
      const content = { subject: 'New Subject', html_body: '<p>New Body</p>', text_body: 'New Body' };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { content, is_customized: true } }),
      });

      await emailTemplateApi.update('invitation', content);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/email-templates/invitation'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify(content),
        })
      );
    });

    it('should reset an email template to default', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { is_customized: false } }),
      });

      const result = await emailTemplateApi.reset('invitation');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/email-templates/invitation'),
        expect.objectContaining({ method: 'DELETE' })
      );
      expect(result.data.is_customized).toBe(false);
    });

    it('should preview an email template', async () => {
      const content = { subject: 'Preview Subject', html_body: '<p>Preview</p>', text_body: 'Preview' };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { subject: 'Rendered Subject', html_body: '<p>Rendered</p>', text_body: 'Rendered' } }),
      });

      const result = await emailTemplateApi.preview('invitation', content);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/email-templates/invitation/preview'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify(content),
        })
      );
      expect(result.data.subject).toBe('Rendered Subject');
    });

    it('should send a test email for a template', async () => {
      const request = {
        to_email: 'test@example.com',
        subject: 'Test Subject',
        html_body: '<p>Test</p>',
        text_body: 'Test',
        variables: { name: 'John' },
      };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ success: true, message: 'Test email sent', message_id: 'msg-456' }),
      });

      const result = await emailTemplateApi.sendTestEmail('invitation', request);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/system/email-templates/invitation/send-test'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify(request),
        })
      );
      expect(result.success).toBe(true);
    });
  });

  describe('serviceApi additional tests', () => {
    it('should update a service', async () => {
      const input = { name: 'Updated Service', base_url: 'https://updated.com' };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: { id: '123', name: 'Updated Service', redirect_uris: [], logout_uris: [], status: 'active', created_at: '', updated_at: '' },
        }),
      });

      await serviceApi.update('123', input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/services/123'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify(input),
        })
      );
    });

    it('should throw error on delete service failure', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        json: async () => ({ error: 'forbidden', message: 'Cannot delete active service' }),
      });

      await expect(serviceApi.delete('123')).rejects.toThrow('Cannot delete active service');
    });

    it('should throw error on delete client failure', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        json: async () => ({ error: 'not_found', message: 'Client not found' }),
      });

      await expect(serviceApi.deleteClient('svc-123', 'client-456')).rejects.toThrow('Client not found');
    });
  });

  describe('tenantApi additional tests', () => {
    it('should throw error on delete tenant failure', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        json: async () => ({ error: 'conflict', message: 'Tenant has active users' }),
      });

      await expect(tenantApi.delete('123')).rejects.toThrow('Tenant has active users');
    });
  });

  describe('userApi additional tests', () => {
    it('should throw error on remove from tenant failure', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        json: async () => ({ error: 'not_found', message: 'User not in tenant' }),
      });

      await expect(userApi.removeFromTenant('user-123', 'tenant-456')).rejects.toThrow('User not in tenant');
    });

    it('should get user assigned roles', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [{ id: 'role-1', name: 'admin' }] }),
      });

      const result = await rbacApi.getUserAssignedRoles('user-123', 'tenant-456');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/user-123/tenants/tenant-456/assigned-roles')
      );
      expect(result.data).toHaveLength(1);
    });
  });

  describe('rbacApi additional tests', () => {
    it('should throw error on delete role failure', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        json: async () => ({ error: 'conflict', message: 'Role is in use' }),
      });

      await expect(rbacApi.deleteRole('svc-123', 'role-456')).rejects.toThrow('Role is in use');
    });

    it('should throw error on unassign role failure', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        json: async () => ({ error: 'not_found', message: 'Role assignment not found' }),
      });

      await expect(rbacApi.unassignRole('user-123', 'tenant-456', 'role-789')).rejects.toThrow('Role assignment not found');
    });
  });

  // ============================================================================
  // Analytics API
  // ============================================================================

  describe('analyticsApi', () => {
    it('should get login stats without date range', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: { total_logins: 100, successful_logins: 90, failed_logins: 10, unique_users: 50 },
        }),
      });

      await analyticsApi.getStats();

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/analytics/login-stats'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
      // Should NOT contain query params
      expect(mockFetch.mock.calls[0][0]).not.toContain('?');
    });

    it('should get login stats with date range', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { total_logins: 50 } }),
      });

      await analyticsApi.getStats('2024-01-01', '2024-01-31', 'token123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('start=2024-01-01'),
        expect.objectContaining({ headers: expect.objectContaining({ Authorization: 'Bearer token123' }) })
      );
      expect(mockFetch.mock.calls[0][0]).toContain('end=2024-01-31');
    });

    it('should list login events with pagination', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: [],
          pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
        }),
      });

      await analyticsApi.listEvents(2, 25);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/analytics/login-events?limit=25&offset=25'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should list login events with defaults', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [], pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 } }),
      });

      await analyticsApi.listEvents();

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('limit=50&offset=0'),
        expect.any(Object)
      );
    });
  });

  // ============================================================================
  // Webhook API
  // ============================================================================

  describe('webhookApi', () => {
    it('should list webhooks for tenant', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [] }),
      });

      await webhookApi.list('tenant-1');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/webhooks'),
        expect.objectContaining({ headers: expect.any(Object) })
      );
    });

    it('should get a webhook by ID', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { id: 'wh-1', name: 'Test Webhook' } }),
      });

      const result = await webhookApi.get('tenant-1', 'wh-1');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/webhooks/wh-1'),
        expect.any(Object)
      );
      expect(result.data.name).toBe('Test Webhook');
    });

    it('should create a webhook', async () => {
      const input = { name: 'New Hook', url: 'https://example.com/hook', events: ['user.created'], enabled: true };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { id: 'wh-2', ...input } }),
      });

      await webhookApi.create('tenant-1', input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/webhooks'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify(input),
        })
      );
    });

    it('should update a webhook', async () => {
      const input = { name: 'Updated Hook' };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { id: 'wh-1', name: 'Updated Hook' } }),
      });

      await webhookApi.update('tenant-1', 'wh-1', input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/webhooks/wh-1'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify(input),
        })
      );
    });

    it('should delete a webhook', async () => {
      mockFetch.mockResolvedValue({ ok: true });

      await webhookApi.delete('tenant-1', 'wh-1');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/webhooks/wh-1'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });

    it('should throw error on delete webhook failure', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        json: async () => ({ error: 'not_found', message: 'Webhook not found' }),
      });

      await expect(webhookApi.delete('tenant-1', 'wh-1')).rejects.toThrow('Webhook not found');
    });

    it('should test a webhook', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { success: true, status_code: 200, response_time_ms: 150 } }),
      });

      const result = await webhookApi.test('tenant-1', 'wh-1');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/webhooks/wh-1/test'),
        expect.objectContaining({ method: 'POST' })
      );
      expect(result.data.success).toBe(true);
    });

    it('should regenerate webhook secret', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { id: 'wh-1', secret: 'new-secret' } }),
      });

      const result = await webhookApi.regenerateSecret('tenant-1', 'wh-1');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/webhooks/wh-1/regenerate-secret'),
        expect.objectContaining({ method: 'POST' })
      );
      expect(result.data.secret).toBe('new-secret');
    });
  });

  // ============================================================================
  // Tenant-Service API
  // ============================================================================

  describe('tenantServiceApi', () => {
    it('should list services for tenant', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [{ id: 's1', name: 'App', enabled: true }] }),
      });

      await tenantServiceApi.listServices('tenant-1');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/services'),
        expect.any(Object)
      );
    });

    it('should toggle service', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [] }),
      });

      await tenantServiceApi.toggleService('tenant-1', 'service-1', true);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/services'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ service_id: 'service-1', enabled: true }),
        })
      );
    });

    it('should get enabled services', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [{ id: 's1', enabled: true }] }),
      });

      await tenantServiceApi.getEnabledServices('tenant-1');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/services/enabled'),
        expect.any(Object)
      );
    });
  });

  // ============================================================================
  // Security Alert API
  // ============================================================================

  describe('securityAlertApi', () => {
    it('should list security alerts', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: [],
          pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
        }),
      });

      await securityAlertApi.list(1, 50);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/security/alerts?limit=50&offset=0'),
        expect.any(Object)
      );
      // Should NOT contain unresolved param
      expect(mockFetch.mock.calls[0][0]).not.toContain('unresolved');
    });

    it('should list unresolved-only security alerts', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [], pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 } }),
      });

      await securityAlertApi.list(1, 50, true);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('unresolved=true'),
        expect.any(Object)
      );
    });

    it('should list alerts with custom pagination', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [], pagination: { page: 2, per_page: 25, total: 0, total_pages: 0 } }),
      });

      await securityAlertApi.list(3, 25);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('limit=25&offset=50'),
        expect.any(Object)
      );
    });

    it('should resolve a security alert', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { id: 'alert-1', resolved_at: '2024-01-01' } }),
      });

      const result = await securityAlertApi.resolve('alert-1');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/security/alerts/alert-1/resolve'),
        expect.objectContaining({ method: 'POST' })
      );
      expect(result.data.resolved_at).toBe('2024-01-01');
    });
  });

  // ============================================================================
  // Password API
  // ============================================================================

  describe('passwordApi', () => {
    it('should call forgot password', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ message: 'Reset email sent' }),
      });

      const result = await passwordApi.forgotPassword('user@test.com');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/auth/forgot-password'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ email: 'user@test.com' }),
        })
      );
      expect(result.message).toBe('Reset email sent');
    });

    it('should call reset password', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ message: 'Password reset successful' }),
      });

      const result = await passwordApi.resetPassword('token-123', 'newPass123!');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/auth/reset-password'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ token: 'token-123', new_password: 'newPass123!' }),
        })
      );
      expect(result.message).toBe('Password reset successful');
    });

    it('should call change password', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ message: 'Password changed' }),
      });

      await passwordApi.changePassword('oldPass', 'newPass', 'access-token');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/me/password'),
        expect.objectContaining({
          method: 'POST',
          headers: expect.objectContaining({ Authorization: 'Bearer access-token' }),
          body: JSON.stringify({ current_password: 'oldPass', new_password: 'newPass' }),
        })
      );
    });

    it('should get password policy', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { min_length: 8, require_uppercase: true } }),
      });

      const result = await passwordApi.getPasswordPolicy('tenant-1');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/password-policy')
      );
      expect(result.data.min_length).toBe(8);
    });

    it('should update password policy', async () => {
      const policy = { min_length: 12, require_symbols: true };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: policy }),
      });

      await passwordApi.updatePasswordPolicy('tenant-1', policy);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants/tenant-1/password-policy'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify(policy),
        })
      );
    });
  });

  // ============================================================================
  // Session API
  // ============================================================================

  describe('sessionApi', () => {
    it('should list user sessions', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [{ id: 's1', is_current: true }] }),
      });

      const result = await sessionApi.listMySessions('token');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/me/sessions'),
        expect.objectContaining({ headers: { Authorization: 'Bearer token' } })
      );
      expect(result.data).toHaveLength(1);
    });

    it('should revoke a session', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ message: 'Session revoked' }),
      });

      await sessionApi.revokeSession('session-1', 'token');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/me/sessions/session-1'),
        expect.objectContaining({
          method: 'DELETE',
          headers: { Authorization: 'Bearer token' },
        })
      );
    });

    it('should revoke other sessions', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ message: 'Other sessions revoked' }),
      });

      await sessionApi.revokeOtherSessions('token');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/me/sessions'),
        expect.objectContaining({
          method: 'DELETE',
          headers: { Authorization: 'Bearer token' },
        })
      );
    });

    it('should force logout a user', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ message: 'User logged out' }),
      });

      await sessionApi.forceLogoutUser('user-1', 'admin-token');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/admin/users/user-1/logout'),
        expect.objectContaining({
          method: 'POST',
          headers: expect.objectContaining({ Authorization: 'Bearer admin-token' }),
        })
      );
    });
  });

  // ============================================================================
  // WebAuthn API
  // ============================================================================

  describe('webauthnApi', () => {
    it('should list passkeys', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [{ id: 'cred-1', credential_type: 'public-key' }] }),
      });

      const result = await webauthnApi.listPasskeys('token');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/me/passkeys'),
        expect.objectContaining({ headers: { Authorization: 'Bearer token' } })
      );
      expect(result.data).toHaveLength(1);
    });

    it('should delete a passkey', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ message: 'Passkey deleted' }),
      });

      await webauthnApi.deletePasskey('cred-1', 'token');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/me/passkeys/cred-1'),
        expect.objectContaining({
          method: 'DELETE',
          headers: { Authorization: 'Bearer token' },
        })
      );
    });

    it('should get register URL', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { url: 'https://example.com/register' } }),
      });

      const result = await webauthnApi.getRegisterUrl('https://app.com/callback', 'token');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/auth/webauthn/register?redirect_uri='),
        expect.objectContaining({ headers: { Authorization: 'Bearer token' } })
      );
      expect(result.data.url).toBe('https://example.com/register');
    });
  });

  // ============================================================================
  // Identity Provider API
  // ============================================================================

  describe('identityProviderApi', () => {
    it('should list identity providers', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [{ alias: 'google', enabled: true }] }),
      });

      await identityProviderApi.list();

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/identity-providers'),
        expect.any(Object)
      );
    });

    it('should get an identity provider', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { alias: 'google', display_name: 'Google' } }),
      });

      const result = await identityProviderApi.get('google');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/identity-providers/google'),
        expect.any(Object)
      );
      expect(result.data.alias).toBe('google');
    });

    it('should create an identity provider', async () => {
      const input = { alias: 'github', provider_id: 'github', config: { clientId: '123' } };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { ...input, enabled: false } }),
      });

      await identityProviderApi.create(input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/identity-providers'),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify(input),
        })
      );
    });

    it('should update an identity provider', async () => {
      const input = { display_name: 'Updated Google', enabled: true };

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: { alias: 'google', ...input } }),
      });

      await identityProviderApi.update('google', input);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/identity-providers/google'),
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify(input),
        })
      );
    });

    it('should delete an identity provider', async () => {
      mockFetch.mockResolvedValue({ ok: true });

      await identityProviderApi.delete('google');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/identity-providers/google'),
        expect.objectContaining({ method: 'DELETE' })
      );
    });

    it('should throw error on delete provider failure', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        json: async () => ({ error: 'conflict', message: 'Provider in use' }),
      });

      await expect(identityProviderApi.delete('google')).rejects.toThrow('Provider in use');
    });

    it('should list linked identities', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [{ id: 'li-1', provider_type: 'google' }] }),
      });

      const result = await identityProviderApi.listMyLinkedIdentities('token');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/me/linked-identities'),
        expect.objectContaining({ headers: { Authorization: 'Bearer token' } })
      );
      expect(result.data).toHaveLength(1);
    });

    it('should unlink an identity', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ message: 'Identity unlinked' }),
      });

      await identityProviderApi.unlinkIdentity('li-1', 'token');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/users/me/linked-identities/li-1'),
        expect.objectContaining({
          method: 'DELETE',
          headers: { Authorization: 'Bearer token' },
        })
      );
    });
  });

  // ============================================================================
  // Public Branding API
  // ============================================================================

  describe('publicBrandingApi', () => {
    it('should get public branding without auth', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          data: { primary_color: '#007AFF', allow_registration: true },
        }),
      });

      const result = await publicBrandingApi.get();

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/public/branding')
      );
      expect(result.data.primary_color).toBe('#007AFF');
    });
  });
});
