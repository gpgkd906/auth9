import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import type {
  ConfigOverview,
  ConfigValidationResult,
  ConfigVersionDetail,
  ConfigVersionSummary,
  CreateTaskOptions,
  CreateTaskRequest,
  SaveConfigFormRequest,
  SaveConfigYamlRequest,
  LogChunk,
  TaskEventEnvelope,
  TaskDetail,
  TaskSummary
} from './types';

export const api = {
  bootstrap: () => invoke<{ resumed_task_id: string | null }>('bootstrap'),
  getConfigOverview: () =>
    invoke<ConfigOverview>('get_config_overview'),
  saveConfigFromForm: (payload: SaveConfigFormRequest) =>
    invoke<ConfigOverview>('save_config_from_form', { payload }),
  saveConfigFromYaml: (payload: SaveConfigYamlRequest) =>
    invoke<ConfigOverview>('save_config_from_yaml', { payload }),
  validateConfigYaml: (payload: SaveConfigYamlRequest) =>
    invoke<ConfigValidationResult>('validate_config_yaml', { payload }),
  listConfigVersions: () =>
    invoke<ConfigVersionSummary[]>('list_config_versions'),
  getConfigVersion: (version: number) =>
    invoke<ConfigVersionDetail>('get_config_version', { version }),
  getCreateTaskOptions: () =>
    invoke<CreateTaskOptions>('get_create_task_options'),
  listTasks: () => invoke<TaskSummary[]>('list_tasks'),
  createTask: (payload: CreateTaskRequest) =>
    invoke<TaskSummary>('create_task', { payload }),
  getTaskDetails: (taskId: string) =>
    invoke<TaskDetail>('get_task_details', { task_id: taskId }),
  startTask: (taskId: string) =>
    invoke<TaskSummary>('start_task', { task_id: taskId }),
  pauseTask: (taskId: string) =>
    invoke<TaskSummary>('pause_task', { task_id: taskId }),
  resumeTask: (taskId: string) =>
    invoke<TaskSummary>('resume_task', { task_id: taskId }),
  retryTaskItem: (taskItemId: string) =>
    invoke<TaskSummary>('retry_task_item', { task_item_id: taskItemId }),
  streamTaskLogs: (taskId: string, limit = 300) =>
    invoke<LogChunk[]>('stream_task_logs', { task_id: taskId, limit }),
  subscribeTaskEvents: (handler: (event: TaskEventEnvelope) => void) =>
    listen<TaskEventEnvelope>('task-event', (event) => {
      handler(event.payload);
    })
};
