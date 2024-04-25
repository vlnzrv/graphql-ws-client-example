use std::{future::IntoFuture, time::Duration};

use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue};
use assert_matches::assert_matches;
use graphql_client::GraphQLQuery;
use graphql_ws_client::graphql::StreamingOperation;
use subscription_server::SubscriptionServer;
use tokio::time::sleep;
use futures::StreamExt;

mod subscription_server;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "./src/graphql-client-subscription.graphql",
    schema_path = "./schemas/books.graphql",
    response_derives = "Debug"
)]
struct BooksChanged;

#[tokio::main]
async fn main() {
    let server = SubscriptionServer::start().await;

    sleep(Duration::from_millis(20)).await;

    let mut request = server.websocket_url().into_client_request().unwrap();
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_str("graphql-transport-ws").unwrap(),
    );

    let (connection, _) = tokio_tungstenite::connect_async(request).await.unwrap();

    println!("Connected");

    let (client, actor) = graphql_ws_client::Client::build(connection).await.unwrap();

    tokio::spawn(actor.into_future());

    let stream = client.subscribe(build_query()).await.unwrap();

    sleep(Duration::from_millis(100)).await;

    let updates = [
        subscription_server::BookChanged {
            id: "123".into(),
            book: None,
        },
        subscription_server::BookChanged {
            id: "456".into(),
            book: None,
        },
        subscription_server::BookChanged {
            id: "789".into(),
            book: None,
        },
    ];

    futures::join!(
        async {
            for update in &updates {
                server.send(update.to_owned()).unwrap();
            }
        },
        async {
            let received_updates = stream.take(updates.len()).collect::<Vec<_>>().await;

            for (expected, update) in updates.iter().zip(received_updates) {
                let update = update.unwrap();
                assert_matches!(update.errors, None);
                let data = update.data.unwrap();
                assert_eq!(data.books.id, expected.id.0);
            }
        }
    );
}

fn build_query() -> graphql_ws_client::graphql::StreamingOperation<BooksChanged> {
    StreamingOperation::new(books_changed::Variables)
}

impl PartialEq<subscription_server::Book> for books_changed::BooksChangedBooksBook {
    fn eq(&self, other: &subscription_server::Book) -> bool {
        self.id == other.id.0 && self.name == other.name && self.author == other.author
    }
}
