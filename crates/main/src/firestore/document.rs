use crate::firestore::FirestoreCollection;

/// `FirestoreCollection` `C` のドキュメントを 1 件取得する。
///
/// パス構築 (`C::document_path`) と deserialize (`C::Schema`) を同じ `C` に束ねるため、
/// パスとスキーマの取り違えが構造的に起きない。
pub(crate) async fn get<C: FirestoreCollection>(
    firestore: &bouzuya_firestore_client::Firestore,
    parent: &C::ParentDocumentId,
    id: &C::DocumentId,
) -> anyhow::Result<Option<C::Schema>> {
    let doc_ref = firestore
        .doc(C::document_path(parent, id))
        .map_err(|e| anyhow::anyhow!(e))?;
    let snapshot = doc_ref.get().await.map_err(|e| anyhow::anyhow!(e))?;
    if !snapshot.exists() {
        return Ok(None);
    }
    let data = snapshot
        .data::<C::Schema>()
        .ok_or_else(|| anyhow::anyhow!("document data is missing"))?
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(Some(data))
}
