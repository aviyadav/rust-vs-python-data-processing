// src/lib.rs
// This amazing macro just generates all the necessary boilerplate so our Wasm component
// can properly implement the `wasi:http/proxy` world's `incoming-handler` bit. It's magic!
wasi::http::proxy::export!(Component);

use std::collections::HashMap;
use wasi::http::types::{
    Fields, // Now we use `Fields` for headers, which is kinda neat
    IncomingRequest,
    OutgoingBody,
    OutgoingResponse,
    ResponseOutparam,
};

// This is just our little placeholder struct to hold our HTTP service logic
struct Component;

// Here's where we actually make our Component act like a `wasi:http/incoming-handler`.
// The `handle` function? That's the VIP entrance for every single HTTP request that comes in.
impl wasi::exports::http::incoming_handler::Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        // I always like to print out what's coming in, helps with debugging, you know?
        // `eprintln!` sends to standard error, which Wasmtime usually picks up.
        eprintln!(
            "Incoming request: {:?} {}",
            request.method(),
            request.path_with_query().unwrap_or_default()
        );

        // Time to get our response headers ready.
        let headers = Fields::new();
        // Header values are byte vectors, and each name can have multiple, hence the slice!
        headers
            .set("Content-Type", &[b"text/plain".to_vec()])
            .unwrap();

        // Split the path from the query string, so routing still works when the
        // URL has one (like `/greet?name=AwesomeDev`).
        let path_with_query = request.path_with_query().unwrap_or_default();
        let (path, query) = match path_with_query.split_once('?') {
            Some((p, q)) => (p, Some(q.to_string())),
            None => (path_with_query.as_str(), None),
        };

        // Now for the fun part: deciding what to send back based on the URL path.
        let body_content = match path {
            "/hello" => "Hello from WebAssembly with Rust!".to_string(),
            "/greet" => {
                // Let's try to grab a name from the URL, like `/greet?name=AwesomeDev`.
                // The `url` crate is seriously helpful for parsing these!
                if let Some(query_string) = query {
                    let params: HashMap<String, String> =
                        url::form_urlencoded::parse(query_string.as_bytes())
                            .into_owned()
                            .collect();
                    if let Some(name) = params.get("name") {
                        format!("Greetings, {} from Wasm!", name)
                    } else {
                        "Greetings, stranger!".to_string()
                    }
                } else {
                    "Greetings, stranger!".to_string()
                }
            }
            "/info" => {
                let mut info = String::new();
                info.push_str("Wasm HTTP Service Info:\n");
                info.push_str(&format!("Method: {:?}\n", request.method()));
                info.push_str(&format!("Path: {:?}\n", request.path_with_query()));
                // Wanna see an example of reading a header? Here's how you grab the User-Agent!
                // Note: `get` returns *all* values for the name, so we just take the first.
                let user_agents = request.headers().get("User-Agent");
                if let Some(user_agent_field) = user_agents.first() {
                    if let Ok(user_agent) = String::from_utf8(user_agent_field.clone()) {
                        info.push_str(&format!("User-Agent: {}\n", user_agent));
                    }
                }
                info
            }
            // If nothing else matches, this is our default friendly message!
            _ => "Welcome to your Rust Wasm HTTP Service!".to_string(),
        };

        // Okay, making our response object. `new` takes our headers, and the
        // status code defaults to 200 OK. A good sign, right?
        let response = OutgoingResponse::new(headers);
        // Grab the body *before* handing the response off to the host.
        let body = response.body().unwrap();
        // Finally, tell the world (or rather, the host runtime) what our response is.
        ResponseOutparam::set(response_out, Ok(response));
        // And here's where our message actually goes!
        let stream = body.write().unwrap();
        stream
            .blocking_write_and_flush(body_content.as_bytes())
            .unwrap();
        // The stream is a child of the body: drop it before finishing, or finishing traps.
        drop(stream);
        OutgoingBody::finish(body, None).unwrap();
    }
}
