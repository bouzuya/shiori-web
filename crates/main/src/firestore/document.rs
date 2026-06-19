use std::marker::PhantomData;

use crate::FirestoreCollection;

/// `FirestoreCollection` `C` に束ねられた型付きのドキュメント参照。
///
/// # 背景 (なぜこれがあるか)
///
/// 素の `bouzuya_firestore_client` では、書き込みは
/// `transaction.set(doc_ref, data)` のように「どこに (`doc_ref`)」と
/// 「何を (`data`)」が独立した引数になっている。`set` は任意の
/// `Serialize` を受け取るため、別コレクションのスキーマを誤ったパスへ
/// 書き込んでもコンパイルが通り、エラーも出ないまま壊れる
/// (例: `users/{id}` に `GoogleUserIdDocumentData` を書く)。
///
/// `DocumentRef<C>` はパス (`C::document_path`) と
/// スキーマ (`C::Schema`) を同じ `C` に束ねるので、`set` / `create` は
/// その `C` のスキーマしか受け付けず、取り違えが**コンパイルエラー**になる。
///
/// # これは必須の入り口ではない
///
/// これは素のクライアント API の上に被せた任意の安全レイヤであって、
/// すべての読み書きがここを通らなければならない、という性格のものではない。
/// `firestore.doc(..)` や `transaction.set(..)` を直接使う選択肢は残っている。
/// パスとスキーマの対応を型で検査したい箇所
/// (特にトランザクション内の書き込み) で使えば良い。
#[derive(Clone)]
pub(crate) struct DocumentRef<C> {
    inner: bouzuya_firestore_client::DocumentReference,
    _marker: PhantomData<C>,
}

impl<C: FirestoreCollection> DocumentRef<C> {
    pub(crate) fn new(
        firestore: &bouzuya_firestore_client::Firestore,
        parent: &C::ParentDocumentId,
        id: &C::DocumentId,
    ) -> anyhow::Result<Self> {
        let inner = firestore
            .doc(C::document_path(parent, id))
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(Self {
            inner,
            _marker: PhantomData,
        })
    }

    pub(crate) fn create(
        &self,
        transaction: &mut bouzuya_firestore_client::Transaction,
        data: &C::Schema,
    ) -> Result<(), bouzuya_firestore_client::Error> {
        transaction.create(&self.inner, data)
    }

    pub(crate) fn delete(
        &self,
        transaction: &mut bouzuya_firestore_client::Transaction,
        precondition: bouzuya_firestore_client::Precondition,
    ) -> Result<(), bouzuya_firestore_client::Error> {
        transaction.delete(&self.inner, precondition)
    }

    pub(crate) async fn get(&self) -> anyhow::Result<Option<C::Schema>> {
        let snapshot = self.inner.get().await.map_err(|e| anyhow::anyhow!(e))?;
        match snapshot.data::<C::Schema>() {
            None => Ok(None),
            Some(result) => Ok(Some(result.map_err(|e| anyhow::anyhow!(e))?)),
        }
    }

    pub(crate) async fn get_in_transaction(
        &self,
        transaction: &mut bouzuya_firestore_client::Transaction,
    ) -> Result<Option<C::Schema>, bouzuya_firestore_client::Error> {
        let snapshot = transaction.get(&self.inner).await?;
        match snapshot.data::<C::Schema>() {
            None => Ok(None),
            Some(result) => Ok(Some(result?)),
        }
    }

    pub(crate) fn set(
        &self,
        transaction: &mut bouzuya_firestore_client::Transaction,
        data: &C::Schema,
    ) -> Result<(), bouzuya_firestore_client::Error> {
        transaction.set(&self.inner, data)
    }
}

/// `FirestoreCollection` を拡張し、コレクション単位の一発読み取りを提供する。
pub(crate) trait FirestoreCollectionExt: FirestoreCollection {
    /// `FirestoreCollection` `Self` のドキュメントを 1 件取得する一発読み取りの薄い wrapper 。
    /// トランザクション外の単発の読み取りに使う。
    async fn get(
        firestore: &bouzuya_firestore_client::Firestore,
        parent: &Self::ParentDocumentId,
        id: &Self::DocumentId,
    ) -> anyhow::Result<Option<Self::Schema>>;
}

impl<C: FirestoreCollection> FirestoreCollectionExt for C {
    async fn get(
        firestore: &bouzuya_firestore_client::Firestore,
        parent: &Self::ParentDocumentId,
        id: &Self::DocumentId,
    ) -> anyhow::Result<Option<Self::Schema>> {
        DocumentRef::<C>::new(firestore, parent, id)?.get().await
    }
}
