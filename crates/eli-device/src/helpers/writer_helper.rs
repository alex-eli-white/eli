
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use eli_protocol::edge_vanilla::result_vanilla::{EdgeError, EdgeResult};
use eli_protocol::edge_vanilla::scanner::msg_vanilla::EdgeEvent;

pub async fn writer_task_helper(mut write_half: OwnedWriteHalf, mut event_rx : Receiver::<EdgeEvent>) ->  EdgeResult<()> {
    while let Some(event) = event_rx.recv().await {
        let line = serde_json::to_string(&event)?;
        write_half.write_all(line.as_bytes()).await?;
        write_half.write_all(b"\n").await?;
    }

    Ok::<(), EdgeError>(())
}