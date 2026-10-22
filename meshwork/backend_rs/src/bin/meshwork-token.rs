//! Issues, lists and revokes actor API tokens, straight in FalkorDB.
//!
//!     meshwork-token create <actor id or name> [--label <text>]
//!     meshwork-token list [<actor id or name>]
//!     meshwork-token revoke <token id>
//!
//! `create` prints only the token on stdout, so it can be captured:
//! `docker compose exec -T backend meshwork-token create claude`. Uses the backend's
//! `FALKORDB_URL` and `FALKORDB_GRAPH`.

use backend_rs::{auth, connection, error::AppError};

const USAGE: &str = "usage:
  meshwork-token create <actor id or name> [--label <text>]
  meshwork-token list [<actor id or name>]
  meshwork-token revoke <token id>";

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

fn describe(e: AppError) -> String {
    match e {
        AppError::Database(m)
        | AppError::NotFound(m)
        | AppError::BadRequest(m)
        | AppError::Conflict(m)
        | AppError::Unauthorized(m)
        | AppError::Internal(m) => m,
    }
}

enum Command {
    Create { actor: String, label: String },
    List { actor: Option<String> },
    Revoke { id: String },
}

fn parse(args: &[String]) -> Option<Command> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    Some(match args.as_slice() {
        ["create", actor] => Command::Create {
            actor: actor.to_string(),
            label: String::new(),
        },
        ["create", actor, "--label", label @ ..] if !label.is_empty() => Command::Create {
            actor: actor.to_string(),
            label: label.join(" "),
        },
        ["list"] => Command::List { actor: None },
        ["list", actor] => Command::List {
            actor: Some(actor.to_string()),
        },
        ["revoke", id] => Command::Revoke { id: id.to_string() },
        _ => return None,
    })
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = parse(&args).unwrap_or_else(|| fail(USAGE));
    let uri =
        std::env::var("FALKORDB_URL").unwrap_or_else(|_| "falkor://falkordb:6379".to_string());
    let graph_name = std::env::var("FALKORDB_GRAPH").unwrap_or_else(|_| "meshwork".to_string());
    let client = connection::connect(&uri)
        .await
        .unwrap_or_else(|e| fail(format!("cannot connect to {uri}: {}", describe(e))));
    let mut graph = client.select_graph(graph_name);
    connection::ensure_indexes(&mut graph)
        .await
        .unwrap_or_else(|e| fail(describe(e)));

    let result = match command {
        Command::Create { actor, label } => auth::issue_token(&mut graph, &actor, &label)
            .await
            .map(|(token, id, actor)| {
                eprintln!(
                    "Token {id} for {} ({}). Shown once, store it now:",
                    actor.name, actor.id
                );
                println!("{token}");
            }),
        Command::List { actor } => {
            auth::list_tokens(&mut graph, actor.as_deref())
                .await
                .map(|tokens| {
                    if tokens.is_empty() {
                        eprintln!("No tokens.");
                    }
                    for t in tokens {
                        println!("{}\t{}\t{}\t{}", t.id, t.actor_name, t.created_at, t.label);
                    }
                })
        }
        Command::Revoke { id } => auth::revoke_token(&mut graph, &id)
            .await
            .map(|()| eprintln!("Token {id} revoked.")),
    };
    if let Err(e) = result {
        fail(describe(e));
    }
}
