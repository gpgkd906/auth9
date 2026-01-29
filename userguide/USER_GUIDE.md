# Auth9 Administrator Guide

本指南详细说明了 Auth9 管理系统的主要操作流程，包括租户管理、服务注册、角色权限管理以及用户关联配置。

## 1. 租户管理 (Tenant Management)

租户是系统的核心隔离单位。

### 创建租户
1. 点击左侧导航栏的 **Tenants**。
2. 点击右上角的 **+ Create Tenant** 按钮。
3. 填写以下信息：
   - **Name**: 租户显示名称 (例如 "Acme Corp")。
   - **Slug**: 租户唯一标识符，用于 URL (例如 "acme")。必须唯一。
   - **Logo URL**: (可选) 租户 Logo 图片地址。
4. 点击 **Create** 完成创建。

### 设置租户
当前租户设置主要通过 "Edit" 功能进行：
1. 在租户列表中找到目标租户。
2. 点击右侧的 `...` 菜单，选择 **Edit**。
3. 修改名称、Slug 或 Logo URL。
4. 点击 **Save Changes**。
*(注：更高级的租户特定设置将在后续版本中通过专门的 Settings 页面提供)*

---

## 2. 服务注册 (Service Registration)

服务 (Service) 代表接入 Auth9 的应用程序 (OIDC Client)。

### 注册新服务
1. 点击左侧导航栏的 **Services**。
2. 点击右上角的 **+ Register Service** 按钮。
3. 填写信息：
   - **Service Name**: 应用名称。
   - **Client ID**: (可选) 自定义 Client ID，留空则自动生成。
   - **Base URL**: 应用的主 URL (例如 `https://myapp.com`)。
   - **Redirect URIs**: OIDC 回调地址 (例如 `https://myapp.com/callback`)。多个地址用逗号分隔。
   - **Logout URIs**: 登出回调地址。
4. 点击 **Register**。

### 获取 Client ID 和 Client Secret
1. 在 Services 列表中，找到刚创建的服务。
2. 点击右侧菜单的 **Edit**。
3. 在弹出的对话框底部 "Client Credentials" 区域：
   - **Client ID**: 直接显示在界面上。
   - **Client Secret**: 点击 **Regenerate Client Secret** 按钮。
     - **警告**: 此操作会使旧的 Secret 失效。
     - 系统会弹出一个对话框显示新的 Secret。**请立即复制保存**，关闭对话框后将无法再次查看 Secret。

---

## 3. 角色与权限管理 (RBAC)

角色 (Role) 是基于服务 (Service) 定义的。租户本身不定义角色，而是使用服务定义的角色。

### 定义角色
1. 点击左侧导航栏的 **Roles**。
2. 页面会列出所有已注册的服务。
3. 找到目标服务，点击该服务区块右上角的 **+ Add Role**。
4. 输入：
   - **Role Name**: 角色标识 (例如 `admin`, `editor`).
   - **Description**: 角色描述。
5. 点击 **Create**。

*(注：权限 Permission 目前主要由开发人员在后端预设，管理员主要负责组合 Role)*

---

## 4. 用户与权限关联 (User Provisioning)

将用户加入租户并分配角色。

### 邀请/创建用户
1. 点击左侧导航栏的 **Users**。
2. *(当前版本用户通过 OIDC 登录自动创建，或通过管理员 API 创建，暂无前端创建按钮，后续添加)*
3. 在用户列表中找到目标用户。

### 关联用户到租户 (Assign Tenant)
1. 点击用户行右侧的 `...` 菜单。
2. 选择 **Manage Tenants**。
3. 在 "Add to Tenant" 区域：
   - 选择 **Tenant**。
   - 选择初始 **Role** (如 `member`)。
   - 点击 **Add**。
4. 用户现在已加入该租户列表。

### 分配服务角色 (Assign Roles)
在 **Manage Tenants** 对话框中，针对已加入的租户：
1. 点击 **Roles** 按钮。
2. 在弹出框中：
   - 选择 **Service** (即该角色所属的应用)。
   - 系统会列出该服务下可用的所有角色。
   - 勾选通过复选框 (Checkbox) 为用户分配的角色。
3. 点击 **Save Roles**。
4. 用户在此租户下即拥有了该服务对应的角色权限。

---

## 5. 常见问题

- **无法登录？** 检查 Redirect URI 是否配置正确。
- **Secret 丢失？** 使用 **Regenerate Client Secret** 生成新的。
