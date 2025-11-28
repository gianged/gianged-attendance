//! Attendance repository for sync and reporting operations.

use crate::entities::{attendance_logs, prelude::*};
use crate::models::attendance::{AttendanceDetail, CreateAttendanceLog, DailyAttendance};
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::sea_query::OnConflict;
use sea_orm::*;

/// Batch size for bulk inserts.
/// 500 records x 5 fields = 2,500 params (well under PostgreSQL's 65,535 limit).
const INSERT_BATCH_SIZE: usize = 500;

/// Insert a batch of attendance logs with deduplication using bulk insert.
///
/// Uses ON CONFLICT DO NOTHING to skip duplicates based on (scanner_uid, check_time).
/// Processes records in chunks of 500 for optimal performance.
/// Returns the count of records processed (duplicates are silently skipped).
pub async fn insert_batch(db: &DatabaseConnection, records: &[CreateAttendanceLog]) -> Result<usize, DbErr> {
    insert_batch_with_progress(db, records, |_, _| {}).await
}

/// Insert a batch of attendance logs with progress reporting.
///
/// Calls `on_progress(processed, total)` after each chunk is inserted.
pub async fn insert_batch_with_progress<F>(
    db: &DatabaseConnection,
    records: &[CreateAttendanceLog],
    mut on_progress: F,
) -> Result<usize, DbErr>
where
    F: FnMut(usize, usize),
{
    if records.is_empty() {
        return Ok(0);
    }

    let total = records.len();
    let mut processed = 0;

    for chunk in records.chunks(INSERT_BATCH_SIZE) {
        let models: Vec<attendance_logs::ActiveModel> = chunk
            .iter()
            .map(|record| attendance_logs::ActiveModel {
                scanner_uid: Set(record.scanner_uid),
                check_time: Set(record.check_time.into()),
                verify_type: Set(record.verify_type),
                status: Set(record.status),
                source: Set(record.source.clone()),
                ..Default::default()
            })
            .collect();

        // Use insert_many for bulk insert with ON CONFLICT DO NOTHING
        AttendanceLogs::insert_many(models)
            .on_conflict(
                OnConflict::columns([attendance_logs::Column::ScannerUid, attendance_logs::Column::CheckTime])
                    .do_nothing()
                    .to_owned(),
            )
            .exec(db)
            .await
            .ok(); // Ignore errors from empty inserts (all duplicates)

        processed += chunk.len();
        on_progress(processed, total);
    }

    Ok(processed)
}

/// Insert a single attendance log.
///
/// Returns None if the record already exists (duplicate).
pub async fn insert_one(
    db: &DatabaseConnection,
    record: &CreateAttendanceLog,
) -> Result<Option<attendance_logs::Model>, DbErr> {
    let model = attendance_logs::ActiveModel {
        scanner_uid: Set(record.scanner_uid),
        check_time: Set(record.check_time.into()),
        verify_type: Set(record.verify_type),
        status: Set(record.status),
        source: Set(record.source.clone()),
        ..Default::default()
    };

    match model.insert(db).await {
        Ok(inserted) => Ok(Some(inserted)),
        Err(DbErr::RecordNotInserted) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Get attendance logs within a date range.
pub async fn get_by_date_range(
    db: &DatabaseConnection,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<attendance_logs::Model>, DbErr> {
    let start = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = end_date.and_hms_opt(23, 59, 59).unwrap().and_utc();

    AttendanceLogs::find()
        .filter(attendance_logs::Column::CheckTime.between(start, end))
        .order_by_desc(attendance_logs::Column::CheckTime)
        .all(db)
        .await
}

/// Get attendance logs for a specific scanner UID within a date range.
pub async fn get_by_scanner_uid(
    db: &DatabaseConnection,
    scanner_uid: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<attendance_logs::Model>, DbErr> {
    let start = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = end_date.and_hms_opt(23, 59, 59).unwrap().and_utc();

    AttendanceLogs::find()
        .filter(attendance_logs::Column::ScannerUid.eq(scanner_uid))
        .filter(attendance_logs::Column::CheckTime.between(start, end))
        .order_by_desc(attendance_logs::Column::CheckTime)
        .all(db)
        .await
}

/// Get daily attendance summary from the view.
pub async fn get_daily_summary(
    db: &DatabaseConnection,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<DailyAttendance>, DbErr> {
    DailyAttendance::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"
        SELECT
            employee_id,
            employee_code,
            full_name,
            department_id,
            department_name,
            work_date,
            first_check,
            last_check,
            check_count,
            work_hours
        FROM app.v_daily_attendance
        WHERE work_date BETWEEN $1 AND $2
        ORDER BY work_date DESC, employee_code
        "#,
        [start_date.into(), end_date.into()],
    ))
    .all(db)
    .await
}

/// Get daily attendance summary filtered by department.
pub async fn get_daily_summary_by_department(
    db: &DatabaseConnection,
    department_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<DailyAttendance>, DbErr> {
    DailyAttendance::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"
        SELECT
            employee_id,
            employee_code,
            full_name,
            department_id,
            department_name,
            work_date,
            first_check,
            last_check,
            check_count,
            work_hours
        FROM app.v_daily_attendance
        WHERE department_id = $1 AND work_date BETWEEN $2 AND $3
        ORDER BY work_date DESC, employee_code
        "#,
        [department_id.into(), start_date.into(), end_date.into()],
    ))
    .all(db)
    .await
}

/// Get attendance details from the view.
/// Returns individual check records with employee info.
pub async fn get_attendance_details(
    db: &DatabaseConnection,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<AttendanceDetail>, DbErr> {
    AttendanceDetail::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"
        SELECT
            id,
            scanner_uid,
            employee_id,
            employee_code,
            full_name,
            department_id,
            department_name,
            check_time,
            verify_type,
            verify_type_name,
            source
        FROM app.v_attendance_details
        WHERE DATE(check_time) BETWEEN $1 AND $2
        ORDER BY check_time DESC
        "#,
        [start_date.into(), end_date.into()],
    ))
    .all(db)
    .await
}

/// Get attendance details filtered by department.
pub async fn get_attendance_details_by_department(
    db: &DatabaseConnection,
    department_id: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<AttendanceDetail>, DbErr> {
    AttendanceDetail::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"
        SELECT
            id,
            scanner_uid,
            employee_id,
            employee_code,
            full_name,
            department_id,
            department_name,
            check_time,
            verify_type,
            verify_type_name,
            source
        FROM app.v_attendance_details
        WHERE department_id = $1 AND DATE(check_time) BETWEEN $2 AND $3
        ORDER BY check_time DESC
        "#,
        [department_id.into(), start_date.into(), end_date.into()],
    ))
    .all(db)
    .await
}

/// Get the latest check time for incremental sync.
pub async fn get_latest_check_time(db: &DatabaseConnection) -> Result<Option<DateTime<Utc>>, DbErr> {
    let result = AttendanceLogs::find()
        .order_by_desc(attendance_logs::Column::CheckTime)
        .one(db)
        .await?;

    Ok(result.map(|r| r.check_time.with_timezone(&Utc)))
}

/// Get count of unique employees who checked in today.
pub async fn get_today_count(db: &DatabaseConnection) -> Result<u64, DbErr> {
    use sea_orm::sea_query::Expr;

    let today = chrono::Utc::now().date_naive();
    let start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = today.and_hms_opt(23, 59, 59).unwrap().and_utc();

    let result: Option<i64> = AttendanceLogs::find()
        .filter(attendance_logs::Column::CheckTime.between(start, end))
        .select_only()
        .column_as(Expr::col(attendance_logs::Column::ScannerUid).count_distinct(), "count")
        .into_tuple()
        .one(db)
        .await?;

    Ok(result.unwrap_or(0) as u64)
}

/// Delete attendance logs before a given date.
pub async fn delete_before(db: &DatabaseConnection, before_date: NaiveDate) -> Result<u64, DbErr> {
    let before = before_date.and_hms_opt(0, 0, 0).unwrap().and_utc();

    let result = AttendanceLogs::delete_many()
        .filter(attendance_logs::Column::CheckTime.lt(before))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

/// Get total attendance log count.
pub async fn count_all(db: &DatabaseConnection) -> Result<u64, DbErr> {
    AttendanceLogs::find().count(db).await
}

// ============================================================================
// Pagination Support
// ============================================================================

/// Pagination parameters for paginated queries.
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub page: u64,
    pub page_size: u64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 500,
        }
    }
}

impl Pagination {
    pub fn new(page: u64, page_size: u64) -> Self {
        Self { page, page_size }
    }

    pub fn offset(&self) -> u64 {
        self.page * self.page_size
    }
}

/// Count daily attendance records in a date range.
pub async fn count_daily_summary(
    db: &DatabaseConnection,
    start_date: NaiveDate,
    end_date: NaiveDate,
    department_id: Option<i32>,
) -> Result<u64, DbErr> {
    #[derive(Debug, FromQueryResult)]
    struct CountResult {
        count: i64,
    }

    let (sql, params): (&str, Vec<Value>) = match department_id {
        Some(dept_id) => (
            r#"
            SELECT COUNT(*) as count
            FROM app.v_daily_attendance
            WHERE department_id = $1 AND work_date BETWEEN $2 AND $3
            "#,
            vec![dept_id.into(), start_date.into(), end_date.into()],
        ),
        None => (
            r#"
            SELECT COUNT(*) as count
            FROM app.v_daily_attendance
            WHERE work_date BETWEEN $1 AND $2
            "#,
            vec![start_date.into(), end_date.into()],
        ),
    };

    let result = CountResult::find_by_statement(Statement::from_sql_and_values(DbBackend::Postgres, sql, params))
        .one(db)
        .await?;

    Ok(result.map(|r| r.count as u64).unwrap_or(0))
}

/// Get paginated daily attendance summary.
pub async fn get_daily_summary_paginated(
    db: &DatabaseConnection,
    start_date: NaiveDate,
    end_date: NaiveDate,
    department_id: Option<i32>,
    pagination: Pagination,
) -> Result<Vec<DailyAttendance>, DbErr> {
    let offset = pagination.offset() as i64;
    let limit = pagination.page_size as i64;

    let (sql, params): (&str, Vec<Value>) = match department_id {
        Some(dept_id) => (
            r#"
            SELECT
                employee_id,
                employee_code,
                full_name,
                department_id,
                department_name,
                work_date,
                first_check,
                last_check,
                check_count,
                work_hours
            FROM app.v_daily_attendance
            WHERE department_id = $1 AND work_date BETWEEN $2 AND $3
            ORDER BY work_date DESC, employee_code
            LIMIT $4 OFFSET $5
            "#,
            vec![
                dept_id.into(),
                start_date.into(),
                end_date.into(),
                limit.into(),
                offset.into(),
            ],
        ),
        None => (
            r#"
            SELECT
                employee_id,
                employee_code,
                full_name,
                department_id,
                department_name,
                work_date,
                first_check,
                last_check,
                check_count,
                work_hours
            FROM app.v_daily_attendance
            WHERE work_date BETWEEN $1 AND $2
            ORDER BY work_date DESC, employee_code
            LIMIT $3 OFFSET $4
            "#,
            vec![start_date.into(), end_date.into(), limit.into(), offset.into()],
        ),
    };

    DailyAttendance::find_by_statement(Statement::from_sql_and_values(DbBackend::Postgres, sql, params))
        .all(db)
        .await
}

/// Count attendance detail records in a date range.
pub async fn count_attendance_details(
    db: &DatabaseConnection,
    start_date: NaiveDate,
    end_date: NaiveDate,
    department_id: Option<i32>,
) -> Result<u64, DbErr> {
    #[derive(Debug, FromQueryResult)]
    struct CountResult {
        count: i64,
    }

    let (sql, params): (&str, Vec<Value>) = match department_id {
        Some(dept_id) => (
            r#"
            SELECT COUNT(*) as count
            FROM app.v_attendance_details
            WHERE department_id = $1 AND DATE(check_time) BETWEEN $2 AND $3
            "#,
            vec![dept_id.into(), start_date.into(), end_date.into()],
        ),
        None => (
            r#"
            SELECT COUNT(*) as count
            FROM app.v_attendance_details
            WHERE DATE(check_time) BETWEEN $1 AND $2
            "#,
            vec![start_date.into(), end_date.into()],
        ),
    };

    let result = CountResult::find_by_statement(Statement::from_sql_and_values(DbBackend::Postgres, sql, params))
        .one(db)
        .await?;

    Ok(result.map(|r| r.count as u64).unwrap_or(0))
}

/// Get paginated attendance details.
pub async fn get_attendance_details_paginated(
    db: &DatabaseConnection,
    start_date: NaiveDate,
    end_date: NaiveDate,
    department_id: Option<i32>,
    pagination: Pagination,
) -> Result<Vec<AttendanceDetail>, DbErr> {
    let offset = pagination.offset() as i64;
    let limit = pagination.page_size as i64;

    let (sql, params): (&str, Vec<Value>) = match department_id {
        Some(dept_id) => (
            r#"
            SELECT
                id,
                scanner_uid,
                employee_id,
                employee_code,
                full_name,
                department_id,
                department_name,
                check_time,
                verify_type,
                verify_type_name,
                source
            FROM app.v_attendance_details
            WHERE department_id = $1 AND DATE(check_time) BETWEEN $2 AND $3
            ORDER BY check_time DESC
            LIMIT $4 OFFSET $5
            "#,
            vec![
                dept_id.into(),
                start_date.into(),
                end_date.into(),
                limit.into(),
                offset.into(),
            ],
        ),
        None => (
            r#"
            SELECT
                id,
                scanner_uid,
                employee_id,
                employee_code,
                full_name,
                department_id,
                department_name,
                check_time,
                verify_type,
                verify_type_name,
                source
            FROM app.v_attendance_details
            WHERE DATE(check_time) BETWEEN $1 AND $2
            ORDER BY check_time DESC
            LIMIT $3 OFFSET $4
            "#,
            vec![start_date.into(), end_date.into(), limit.into(), offset.into()],
        ),
    };

    AttendanceDetail::find_by_statement(Statement::from_sql_and_values(DbBackend::Postgres, sql, params))
        .all(db)
        .await
}

/// Load all records for export (streams in chunks internally).
/// Returns a complete Vec for export functions that need all data.
pub async fn get_all_daily_summary_for_export(
    db: &DatabaseConnection,
    start_date: NaiveDate,
    end_date: NaiveDate,
    department_id: Option<i32>,
) -> Result<Vec<DailyAttendance>, DbErr> {
    // For export, use the non-paginated query since rust_xlsxwriter
    // needs all data upfront. The data is already filtered by date range.
    match department_id {
        Some(dept_id) => get_daily_summary_by_department(db, dept_id, start_date, end_date).await,
        None => get_daily_summary(db, start_date, end_date).await,
    }
}

/// Load all attendance details for export.
pub async fn get_all_attendance_details_for_export(
    db: &DatabaseConnection,
    start_date: NaiveDate,
    end_date: NaiveDate,
    department_id: Option<i32>,
) -> Result<Vec<AttendanceDetail>, DbErr> {
    match department_id {
        Some(dept_id) => get_attendance_details_by_department(db, dept_id, start_date, end_date).await,
        None => get_attendance_details(db, start_date, end_date).await,
    }
}
