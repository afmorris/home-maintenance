//! Integration tests for the repository layer, defaulting to SQLite.
//
// Tests must run single-threaded (`--test-threads=1`) because each test
// rewrites `DATA_DIR` and initializes its own SQLite database.

use chrono::NaiveDate;
use home_maintenance::db::{Db, init_pool};
use home_maintenance::repo::assets::{
    AssetInput, archive_asset, create_asset, get_asset, list_assets, update_asset,
};
use home_maintenance::repo::locations::{
    LocationInput, create_location, delete_location, get_location, list_locations,
};
use home_maintenance::repo::log::{LogEntryInput, get_log_entry, list_log_entries};
use home_maintenance::repo::reminders::{
    complete_task_transaction, list_reminders, list_reminders_with_tasks, snooze_reminder,
    upsert_reminder,
};
use home_maintenance::repo::supplies::{
    SupplyInput, create_supply, delete_supply, get_supply, update_supply,
};
use home_maintenance::repo::tags::{TagInput, attach_tag, create_tag, list_tags_for_entry};
use home_maintenance::repo::tasks::{TaskInput, create_task, delete_task, get_task};
use home_maintenance::util::today_in_tz;
use std::future::Future;
use std::sync::Mutex;

static COUNTER: Mutex<u64> = Mutex::new(0);

async fn with_db<F, Fut, R>(f: F) -> R
where
    F: FnOnce(Db) -> Fut,
    Fut: Future<Output = R>,
{
    let n = {
        let mut c = COUNTER.lock().unwrap();
        *c += 1;
        *c
    };
    let dir = std::path::PathBuf::from(format!("/tmp/hm_test_{n}"));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    unsafe { std::env::set_var("DATA_DIR", dir.to_str().unwrap()) };
    unsafe { std::env::remove_var("DATABASE_URL") };

    // Give the OS a moment to actually complete the directory creation before
    // the app tries to create the SQLite file inside it.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let db = init_pool().await.expect("failed to init pool");
    f(db).await
}

fn uuid() -> String {
    uuid::Uuid::now_v7().to_string()
}

#[tokio::test]
async fn test_location_crud() {
    with_db(|db| async move {
        let id = uuid();
        let input = LocationInput {
            name: "Basement".to_string(),
            kind: "room".to_string(),
            parent_id: None,
        };
        let created = create_location(&db, &id, input).await.unwrap();
        assert_eq!(created.name, "Basement");
        assert_eq!(created.kind, "room");

        let fetched = get_location(&db, &id).await.unwrap();
        assert_eq!(fetched.id, id);

        let list = list_locations(&db).await.unwrap();
        assert!(list.iter().any(|l| l.id == id));

        delete_location(&db, &id).await.unwrap();
        assert!(get_location(&db, &id).await.is_err());
    })
    .await
}

#[tokio::test]
async fn test_asset_crud() {
    with_db(|db| async move {
        let id = uuid();
        let input = AssetInput {
            name: "Water Heater".to_string(),
            location_id: None,
            category: "appliance".to_string(),
            make: Some("Rheem".to_string()),
            model: Some("X100".to_string()),
            serial: None,
            install_date: Some("2020-01-01".to_string()),
            warranty_end: None,
            notes: None,
        };
        let created = create_asset(&db, &id, input).await.unwrap();
        assert_eq!(created.name, "Water Heater");

        let mut update = AssetInput::default();
        update.name = "Updated Heater".to_string();
        let updated = update_asset(&db, &id, update).await.unwrap();
        assert_eq!(updated.name, "Updated Heater");
        assert_eq!(updated.make, Some("Rheem".to_string()));

        archive_asset(&db, &id).await.unwrap();
        let list = list_assets(&db).await.unwrap();
        assert!(!list.iter().any(|a| a.id == id));

        let archived = get_asset(&db, &id).await.unwrap();
        assert_eq!(archived.archived, 1);
    })
    .await
}

#[tokio::test]
async fn test_supply_crud() {
    with_db(|db| async move {
        let id = uuid();
        let input = SupplyInput {
            name: "Filter".to_string(),
            spec: Some("20x20x1".to_string()),
            purchase_url: None,
            notes: None,
        };
        let created = create_supply(&db, &id, input).await.unwrap();
        assert_eq!(created.name, "Filter");

        let mut update = SupplyInput::default();
        update.name = "New Filter".to_string();
        let updated = update_supply(&db, &id, update).await.unwrap();
        assert_eq!(updated.name, "New Filter");

        delete_supply(&db, &id).await.unwrap();
        assert!(get_supply(&db, &id).await.is_err());
    })
    .await
}

#[tokio::test]
async fn test_task_and_reminder() {
    with_db(|db| async move {
        let id = uuid();
        let input = TaskInput {
            name: "Test floating task".to_string(),
            schedule_mode: "floating".to_string(),
            interval_value: Some(1),
            interval_unit: Some("month".to_string()),
            ..TaskInput::default()
        };
        let today = today_in_tz("America/New_York");
        let created = create_task(&db, &id, input, today, None).await.unwrap();
        assert_eq!(created.schedule_mode, "floating");

        upsert_reminder(&db, &id, "2025-01-15", None).await.unwrap();
        let reminders = list_reminders(&db, None, None, None).await.unwrap();
        assert!(reminders.iter().any(|r| r.task_id == id));

        snooze_reminder(&db, &id, "2030-01-20").await.unwrap();
        let snoozed = list_reminders(&db, Some("upcoming"), None, None)
            .await
            .unwrap();
        assert!(snoozed.iter().any(|r| r.task_id == id));

        delete_task(&db, &id).await.unwrap();
        assert!(get_task(&db, &id).await.is_err());
    })
    .await
}

#[tokio::test]
async fn test_list_reminders_with_tasks() {
    with_db(|db| async move {
        let asset_id = uuid();
        create_asset(
            &db,
            &asset_id,
            AssetInput {
                name: "Water Heater".to_string(),
                location_id: None,
                category: "appliance".to_string(),
                make: None,
                model: None,
                serial: None,
                install_date: None,
                warranty_end: None,
                notes: None,
            },
        )
        .await
        .unwrap();

        let task_id = uuid();
        let task = TaskInput {
            asset_id: Some(asset_id.clone()),
            name: "Drain water heater".to_string(),
            schedule_mode: "floating".to_string(),
            interval_value: Some(6),
            interval_unit: Some("month".to_string()),
            ..TaskInput::default()
        };
        let today = today_in_tz("America/New_York");
        create_task(&db, &task_id, task, today, None).await.unwrap();
        upsert_reminder(&db, &task_id, "2025-01-01", None)
            .await
            .unwrap();

        let rows = list_reminders_with_tasks(&db, None).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].task_name, "Drain water heater");
        assert_eq!(rows[0].asset_name, Some("Water Heater".to_string()));
    })
    .await
}

#[tokio::test]
async fn test_complete_task_transaction_floating() {
    with_db(|db| async move {
        let task_id = uuid();
        let task = TaskInput {
            name: "Drain water heater".to_string(),
            schedule_mode: "floating".to_string(),
            interval_value: Some(6),
            interval_unit: Some("month".to_string()),
            ..TaskInput::default()
        };
        let today = today_in_tz("America/New_York");
        create_task(&db, &task_id, task, today, None).await.unwrap();
        upsert_reminder(&db, &task_id, "2025-01-01", None)
            .await
            .unwrap();

        let log_id = uuid();
        let completed = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let entry = complete_task_transaction(
            &db,
            &log_id,
            &task_id,
            None,
            "service",
            completed,
            Some(2500),
            Some("Plumber"),
            Some("Self"),
            Some("annual drain"),
        )
        .await
        .unwrap();

        assert_eq!(entry.task_id, Some(task_id.clone()));
        assert_eq!(entry.cost_cents, Some(2500));

        let fetched = get_log_entry(&db, &log_id).await.unwrap();
        assert_eq!(fetched.completed_date, "2025-01-15");

        let entries = list_log_entries(&db, None, Some("service"), None, None, None, None)
            .await
            .unwrap();
        assert!(entries.iter().any(|e| e.id == log_id));
    })
    .await
}

#[tokio::test]
async fn test_task_creation_computes_reminder() {
    with_db(|db| async move {
        let today = today_in_tz("America/New_York");
        let task_id = uuid();
        let input = TaskInput {
            name: "Quarterly filter change".to_string(),
            schedule_mode: "floating".to_string(),
            interval_value: Some(3),
            interval_unit: Some("month".to_string()),
            ..TaskInput::default()
        };
        let created = create_task(&db, &task_id, input, today, None)
            .await
            .unwrap();
        assert_eq!(created.schedule_mode, "floating");

        let reminders = list_reminders(&db, None, None, None).await.unwrap();
        let reminder = reminders
            .iter()
            .find(|r| r.task_id == task_id)
            .expect("reminder should be created");
        let due: NaiveDate = reminder.due_date.parse().unwrap();
        let expected = today
            .checked_add_months(chrono::Months::new(3))
            .expect("valid date");
        assert_eq!(due, expected, "initial due date should be today + 3 months");
    })
    .await
}

#[tokio::test]
async fn test_tag_attachment() {
    with_db(|db| async move {
        let tag_id = uuid();
        create_tag(
            &db,
            &tag_id,
            TagInput {
                name: "HVAC".to_string(),
            },
        )
        .await
        .unwrap();

        let entry_id = uuid();
        let entry = LogEntryInput {
            task_id: None,
            asset_id: None,
            kind: "repair".to_string(),
            scheduled_date: None,
            completed_date: "2025-02-01".to_string(),
            cost_cents: None,
            vendor: None,
            performed_by: None,
            notes: None,
        };
        home_maintenance::repo::log::create_log_entry(&db, &entry_id, entry)
            .await
            .unwrap();

        attach_tag(&db, &entry_id, &tag_id).await.unwrap();
        let tags = list_tags_for_entry(&db, &entry_id).await.unwrap();
        assert!(tags.iter().any(|t| t.name == "HVAC"));
    })
    .await
}
