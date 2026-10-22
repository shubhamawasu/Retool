use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::{Asset, AssetStatus, AssetsPage, CreateAssetRequest, UpdateAssetRequest};

#[derive(Debug, thiserror::Error)]
pub enum AssetsStoreError {
    #[error("Asset name cannot be empty")]
    InvalidName,
    #[error("Asset category was not found")]
    CategoryNotFound,
    #[error("Asset was not found")]
    AssetNotFound,
    #[error("Unable to access assets JSON: {0}")]
    Io(#[from] std::io::Error),
    #[error("Assets JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct AssetsStore {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl Default for AssetsStore {
    fn default() -> Self {
        Self::new(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("data")
                .join("my-assets.json"),
        )
    }
}

impl AssetsStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn get_page(&self) -> Result<AssetsPage, AssetsStoreError> {
        let _guard = self.lock.lock().await;
        self.read_page().await
    }

    pub async fn create_asset(
        &self,
        request: CreateAssetRequest,
    ) -> Result<Asset, AssetsStoreError> {
        let name = request.name.trim();
        if name.is_empty() {
            return Err(AssetsStoreError::InvalidName);
        }

        let _guard = self.lock.lock().await;
        let mut page = self.read_page().await?;
        let category = page
            .categories
            .iter_mut()
            .find(|category| category.id == request.category_id)
            .ok_or(AssetsStoreError::CategoryNotFound)?;
        let asset = Asset {
            id: Uuid::new_v4().to_string(),
            name: name.to_owned(),
            details: request.details,
            value: request.value,
            status: AssetStatus::Undecided,
            status_label: AssetStatus::Undecided.label().to_owned(),
            note: None,
            client_comment: None,
            value_help: None,
            excluded_from_total: false,
        };
        category.assets.push(asset.clone());

        refresh_summary(&mut page);
        self.write_page(&page).await?;
        Ok(asset)
    }

    pub async fn update_asset(
        &self,
        id: &str,
        request: UpdateAssetRequest,
    ) -> Result<Asset, AssetsStoreError> {
        if request.name.as_deref().is_some_and(|name| name.trim().is_empty()) {
            return Err(AssetsStoreError::InvalidName);
        }

        let _guard = self.lock.lock().await;
        let mut page = self.read_page().await?;
        let asset = page
            .categories
            .iter_mut()
            .flat_map(|category| &mut category.assets)
            .find(|asset| asset.id == id)
            .ok_or(AssetsStoreError::AssetNotFound)?;

        if let Some(name) = request.name {
            asset.name = name.trim().to_owned();
        }
        if let Some(details) = request.details {
            asset.details = details;
        }
        if let Some(value) = request.value {
            asset.value = value;
        }
        if let Some(status) = request.status {
            asset.status = status;
            if request.status_label.is_none() {
                asset.status_label = status.label().to_owned();
            }
        }
        if let Some(status_label) = request.status_label {
            asset.status_label = status_label;
        }
        if let Some(note) = request.note {
            asset.note = Some(note);
        }
        if let Some(client_comment) = request.client_comment {
            asset.client_comment = Some(client_comment);
        }
        let updated = asset.clone();

        refresh_summary(&mut page);
        self.write_page(&page).await?;
        Ok(updated)
    }

    pub async fn delete_asset(&self, id: &str) -> Result<(), AssetsStoreError> {
        let _guard = self.lock.lock().await;
        let mut page = self.read_page().await?;
        let Some(category) = page
            .categories
            .iter_mut()
            .find(|category| category.assets.iter().any(|asset| asset.id == id))
        else {
            return Err(AssetsStoreError::AssetNotFound);
        };
        category.assets.retain(|asset| asset.id != id);

        refresh_summary(&mut page);
        self.write_page(&page).await
    }

    async fn read_page(&self) -> Result<AssetsPage, AssetsStoreError> {
        let contents = tokio::fs::read_to_string(&self.path).await?;
        Ok(serde_json::from_str(&contents)?)
    }

    async fn write_page(&self, page: &AssetsPage) -> Result<(), AssetsStoreError> {
        let contents = serde_json::to_vec_pretty(page)?;
        let temp_path = self.path.with_extension(format!("{}.tmp", Uuid::new_v4()));
        tokio::fs::write(&temp_path, contents).await?;
        if let Err(error) = tokio::fs::rename(&temp_path, &self.path).await {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(error.into());
        }
        Ok(())
    }
}

fn refresh_summary(page: &mut AssetsPage) {
    let assets: Vec<&Asset> = page
        .categories
        .iter()
        .flat_map(|category| &category.assets)
        .collect();
    let verified = assets
        .iter()
        .filter(|asset| asset.status == AssetStatus::Verified)
        .count();
    let not_fundable = assets
        .iter()
        .filter(|asset| asset.status == AssetStatus::NotFundable)
        .count();
    let not_funded = assets
        .iter()
        .filter(|asset| asset.status == AssetStatus::NotFunded)
        .count();
    let fundable_total = assets.len().saturating_sub(not_fundable);

    page.funding.verified = verified;
    page.funding.not_fundable = not_fundable;
    page.funding.not_funded_per_client = not_funded;
    page.funding.fundable_total = fundable_total;
    page.funding.left_to_verify = fundable_total.saturating_sub(verified);
    page.funding.percent = if fundable_total == 0 {
        0.0
    } else {
        (verified as f64 * 1000.0 / fundable_total as f64).round() / 10.0
    };
    page.total_value = assets
        .iter()
        .filter(|asset| {
            !asset.excluded_from_total
                && asset.status != AssetStatus::NotFundable
                && asset.status != AssetStatus::NotFunded
        })
        .map(|asset| asset.value)
        .sum();
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_store() -> (AssetsStore, PathBuf) {
        let path = std::env::temp_dir().join(format!("my-assets-{}.json", Uuid::new_v4()));
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/my-assets.json");
        tokio::fs::copy(fixture, &path).await.unwrap();
        (AssetsStore::new(path.clone()), path)
    }

    #[tokio::test]
    async fn create_asset_persists_to_json_and_updates_total() {
        let (store, path) = test_store().await;
        let original_total = store.get_page().await.unwrap().total_value;

        let created = store
            .create_asset(CreateAssetRequest {
                category_id: "cash".into(),
                name: "Test savings account".into(),
                details: "Created by test".into(),
                value: 1250,
            })
            .await
            .unwrap();
        let reloaded = store.get_page().await.unwrap();

        assert!(reloaded.categories[0].assets.iter().any(|asset| asset.id == created.id));
        assert_eq!(reloaded.total_value, original_total + 1250);
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn update_asset_persists_value_to_json_and_recalculates_total() {
        let (store, path) = test_store().await;
        let original_total = store.get_page().await.unwrap().total_value;

        store
            .update_asset(
                "cash-1",
                UpdateAssetRequest {
                    name: None,
                    details: None,
                    value: Some(26000),
                    status: None,
                    status_label: None,
                    note: None,
                    client_comment: Some("Updated in test".into()),
                },
            )
            .await
            .unwrap();
        let reloaded = store.get_page().await.unwrap();
        let updated = &reloaded.categories[0].assets[0];

        assert_eq!(updated.value, 26000);
        assert_eq!(updated.client_comment.as_deref(), Some("Updated in test"));
        assert_eq!(reloaded.total_value, original_total + 1000);
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn delete_asset_removes_it_from_json_and_recalculates_total() {
        let (store, path) = test_store().await;
        let original_total = store.get_page().await.unwrap().total_value;

        store.delete_asset("cash-1").await.unwrap();
        let reloaded = store.get_page().await.unwrap();

        assert!(!reloaded.categories[0].assets.iter().any(|asset| asset.id == "cash-1"));
        assert_eq!(reloaded.total_value, original_total - 25000);
        tokio::fs::remove_file(path).await.unwrap();
    }
}