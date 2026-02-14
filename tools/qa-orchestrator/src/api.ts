import { invoke } from '@tauri-apps/api/tauri';
import type {
  CreateTaskRequest,
  LogChunk,
  TaskDetail,
  TaskSummary
} from './types';

export const api = {
  bootstrap: () => invoke<{ resumed_task_id: string | null }>('bootstrap'),
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
    invoke<LogChunk[]>('stream_task_logs', { task_id: taskId, limit })
};
