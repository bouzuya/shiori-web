use crate::UserId;

#[async_trait::async_trait]
pub trait UserSettingsReader: Send + Sync {
    /// 指定ユーザーの設定を取得する。未保存なら `None`。
    async fn get(
        &self,
        user_id: UserId,
    ) -> anyhow::Result<Option<crate::read_models::UserSettingsView>>;
}
