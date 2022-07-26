//foo

#[macro_export]
macro_rules! create_table {
    ($query_name:ident, $executor:expr) => {
        sqlx::query($query_name).execute(expr).await?
        println!("finished table creation: {}", $query_name);
    };
}
