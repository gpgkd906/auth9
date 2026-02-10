/** Single data response wrapper */
export interface DataResponse<T> {
  data: T;
}

/** Paginated response wrapper */
export interface PaginatedResponse<T> {
  data: T[];
  pagination: Pagination;
}

/** Pagination metadata */
export interface Pagination {
  page: number;
  perPage: number;
  total: number;
  totalPages: number;
}
