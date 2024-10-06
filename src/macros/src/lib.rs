#[macro_export]
macro_rules! ert {
    () => {
        |e| error!(%e)
    };
}
