// TODO: Check if you can get this info from every storage type or just blob storage
// which is how it was previously implemented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub sku_name: String,
    pub kind: String,
}
