// use tokio::sync::broadcast;
//
// use eli_protocol::edge_vanilla::scanner::msg_vanilla::EdgeEvent;
//
// use crate::router::runtime::RouterRuntime;
//
// impl RouterRuntime {
//     pub(crate) async fn spawn_debug_observer(&self) {
//         let tx = {
//             let state = self.state.lock().await;
//             state.broadcaster.clone()
//         };
//
//         let mut rx = tx.subscribe();
//
//         tokio::spawn(async move {
//             loop {
//                 match rx.recv().await {
//                     Ok(event) => match &event.event {
//                         EdgeEvent::Status(_) => {}
//                         other => {
//                             println!(
//                                 "[router] worker={} source={} EVENT {:?}",
//                                 event.worker_id, event.source_id, other
//                             );
//                         }
//                     },
//                     Err(broadcast::error::RecvError::Lagged(count)) => {
//                         eprintln!("[router] debug observer lagged and dropped {count} events");
//                     }
//                     Err(broadcast::error::RecvError::Closed) => break,
//                 }
//             }
//         });
//     }
// }