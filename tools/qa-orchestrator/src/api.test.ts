import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn()
}));

vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: invokeMock
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock
}));

import { api } from './api';

describe('api wrapper', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
    listenMock.mockReset();
    listenMock.mockResolvedValue(vi.fn());
  });

  it('calls bootstrap', async () => {
    await api.bootstrap();
    expect(invokeMock).toHaveBeenCalledWith('bootstrap');
  });

  it('calls config endpoints', async () => {
    await api.getConfigOverview();
    expect(invokeMock).toHaveBeenNthCalledWith(1, 'get_config_overview');

    await api.saveConfigFromForm({ config: {} as never });
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'save_config_from_form', {
      payload: { config: {} }
    });

    await api.saveConfigFromYaml({ yaml: 'runner: {}' });
    expect(invokeMock).toHaveBeenNthCalledWith(3, 'save_config_from_yaml', {
      payload: { yaml: 'runner: {}' }
    });

    await api.validateConfigYaml({ yaml: 'runner: {}' });
    expect(invokeMock).toHaveBeenNthCalledWith(4, 'validate_config_yaml', {
      payload: { yaml: 'runner: {}' }
    });

    await api.listConfigVersions();
    expect(invokeMock).toHaveBeenNthCalledWith(5, 'list_config_versions');

    await api.getConfigVersion(12);
    expect(invokeMock).toHaveBeenNthCalledWith(6, 'get_config_version', {
      version: 12
    });
  });

  it('calls task endpoints', async () => {
    await api.getCreateTaskOptions();
    expect(invokeMock).toHaveBeenNthCalledWith(1, 'get_create_task_options');

    await api.listTasks();
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'list_tasks');

    await api.createTask({ name: 'n', workspace_id: 'w', workflow_id: 'wf' });
    expect(invokeMock).toHaveBeenNthCalledWith(3, 'create_task', {
      payload: { name: 'n', workspace_id: 'w', workflow_id: 'wf' }
    });

    await api.getTaskDetails('task-1');
    expect(invokeMock).toHaveBeenNthCalledWith(4, 'get_task_details', {
      task_id: 'task-1'
    });

    await api.startTask('task-1');
    expect(invokeMock).toHaveBeenNthCalledWith(5, 'start_task', {
      task_id: 'task-1'
    });

    await api.pauseTask('task-1');
    expect(invokeMock).toHaveBeenNthCalledWith(6, 'pause_task', {
      task_id: 'task-1'
    });

    await api.resumeTask('task-1');
    expect(invokeMock).toHaveBeenNthCalledWith(7, 'resume_task', {
      task_id: 'task-1'
    });

    await api.retryTaskItem('item-1');
    expect(invokeMock).toHaveBeenNthCalledWith(8, 'retry_task_item', {
      task_item_id: 'item-1'
    });

    await api.streamTaskLogs('task-1');
    expect(invokeMock).toHaveBeenNthCalledWith(9, 'stream_task_logs', {
      task_id: 'task-1',
      limit: 300
    });

    await api.streamTaskLogs('task-1', 99);
    expect(invokeMock).toHaveBeenNthCalledWith(10, 'stream_task_logs', {
      task_id: 'task-1',
      limit: 99
    });
  });

  it('subscribes task events', async () => {
    const unlisten = vi.fn();
    listenMock.mockResolvedValue(unlisten);
    const handler = vi.fn();

    const unlistenPromise = api.subscribeTaskEvents(handler);
    expect(listenMock).toHaveBeenCalledTimes(1);
    expect(listenMock).toHaveBeenCalledWith('task-event', expect.any(Function));

    const listener = listenMock.mock.calls[0][1] as (event: {
      payload: {
        task_id: string;
        task_item_id: string | null;
        event_type: string;
        payload: Record<string, unknown>;
        ts: string;
      };
    }) => void;
    const payload = {
      task_id: 'task-1',
      task_item_id: null,
      event_type: 'task_started',
      payload: {},
      ts: '2026-02-14T00:00:00Z'
    };

    listener({ payload });
    expect(handler).toHaveBeenCalledWith(payload);
    await expect(unlistenPromise).resolves.toBe(unlisten);
  });
});
