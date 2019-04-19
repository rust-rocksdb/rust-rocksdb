use crate::Transaction;

pub trait TransactionBegin: Sized {
    type WriteOptions: Default;
    type TransactionOptions: Default;
    fn transaction(
        &self,
        write_options: &<Self as TransactionBegin>::WriteOptions,
        tx_options: &<Self as TransactionBegin>::TransactionOptions,
    ) -> Transaction<Self>;

    /// Begins a new optimistic transaction with default options.
    fn transaction_default(&self) -> Transaction<Self> {
        let write_options = Self::WriteOptions::default();
        let transaction_options = Self::TransactionOptions::default();
        self.transaction(&write_options, &transaction_options)
    }
}
