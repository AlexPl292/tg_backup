use tg_backup;
use tg_backup::{start_backup, Opts};

#[tokio::test]
async fn test_add() {
    start_backup(Opts {
        included_chats: vec![1707414104],
        auth: None,
    }).await;
}
