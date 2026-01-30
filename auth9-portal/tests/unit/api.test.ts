import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  tenantApi,
  userApi,
  serviceApi,
  rbacApi,
  auditApi,
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
        expect.stringContaining('/api/v1/tenants?page=1&per_page=20')
      );
    });

    it('should list tenants with custom pagination', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [], pagination: { page: 2, per_page: 50, total: 0, total_pages: 0 } }),
      });

      await tenantApi.list(2, 50);

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/tenants?page=2&per_page=50')
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
        expect.stringContaining('/api/v1/tenants/123')
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
        expect.stringContaining('/api/v1/users?page=1&per_page=20')
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
        expect.stringContaining('/api/v1/users/123/tenants')
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

      await serviceApi.list();

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/services?page=1&per_page=20')
      );
    });

    it('should list services filtered by tenant', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ data: [], pagination: { page: 1, per_page: 20, total: 0, total_pages: 0 } }),
      });

      await serviceApi.list('tenant-123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('tenant_id=tenant-123')
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
        expect.stringContaining('/api/v1/services/service-123/clients')
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
        expect.stringContaining('/api/v1/services/service-123/roles')
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
        expect.stringContaining('/api/v1/services/service-123/permissions')
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
        expect.stringContaining('/api/v1/audit-logs?limit=50&offset=0')
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
        expect.stringContaining('/api/v1/audit-logs?limit=50&offset=100')
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
});
