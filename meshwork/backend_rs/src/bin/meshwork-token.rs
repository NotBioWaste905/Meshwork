//! Issues, lists and revokes actor API tokens, straight in FalkorDB.
//!
//!     meshwork-token create <actor id or name> [--kind human|agent] [--expires-in <days>] [--label <text>]
//!     meshwork-token list [<actor id or name>]
//!     meshwork-token revoke <token id>
//!
//! `create` prints only the token on stdout, so it can be captured:
//! `docker compose exec -T backend meshwork-token create claude`. Uses the backend's
//! `FALKORDB_URL` and `FALKORDB_GRAPH`.
//!
//! `create` with `--kind` creates the actor when there is none by that id or name: the backend
//! requires a token for every request, so on a fresh install the first actor is made here.
//! Without `--kind` an unknown actor is an error, so a typo does not create a stray actor.

use backend_rs::{auth, connection, error::AppError, schemas::ActorKind};

const USAGE: &str = "usage:
  meshwork-token create <actor id or name> [--kind human|agent] [--expires-in <days>] [--label <text>]
    --kind creates the actor if it does not exist; --label takes the rest of the line
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
    Create {
        actor: String,
        label: String,
        kind: Option<ActorKind>,
        days: Option<u32>,
    },
    List {
        actor: Option<String>,
    },
    Revoke {
        id: String,
    },
}

fn parse(args: &[String]) -> Option<Command> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    Some(match args.as_slice() {
        ["create", actor, rest @ ..] => {
            let mut rest: &[&str] = rest;
            let (mut label, mut kind, mut days) = (String::new(), None, None);
            loop {
                match rest {
                    [] => break,
                    ["--kind", k, tail @ ..] => {
                        kind = Some(ActorKind::parse(k)?);
                        rest = tail;
                    }
                    ["--expires-in", d, tail @ ..] => {
                        days = Some(d.parse().ok()?);
                        rest = tail;
                    }
                    ["--label", words @ ..] if !words.is_empty() => {
                        label = words.join(" ");
                        break;
                    }
                    _ => return None,
                }
            }
            Command::Create {
                actor: actor.to_string(),
                label,
                kind,
                days,
            }
        }
        ["list"] => Command::List { actor: None },
        ["list", actor] => Command::List {
            actor: Some(actor.to_string()),
        },
        ["revoke", id] => Command::Revoke { id: id.to_string() },
        _ => return None,
    })
}

async fn create(
    graph: &mut falkordb::AsyncGraph,
    actor: &str,
    label: &str,
    kind: Option<ActorKind>,
    days: Option<u32>,
) -> Result<(), AppError> {
    let actor = match kind {
        Some(kind) => {
            let (actor, created) = auth::ensure_actor(graph, actor, kind).await?;
            if created {
                eprintln!(
                    "Created {} actor {} ({}).",
                    kind.as_str(),
                    actor.name,
                    actor.id
                );
            }
            actor.id
        }
        None => actor.to_string(),
    };
    let (token, id, actor) = auth::issue_expiring_token(graph, &actor, label, days)
        .await
        .map_err(|e| match e {
            AppError::NotFound(m) => AppError::NotFound(format!(
                "{m}; or pass --kind human|agent to create the actor here"
            )),
            e => e,
        })?;
    let lifetime = match days {
        Some(days) => format!("valid for {days} days"),
        None => "valid until revoked".to_string(),
    };
    eprintln!(
        "Issued a token for {} ({}), {lifetime}; its id, for `revoke`, is {id}.\n\
         The token is the mw_… line below (not the id). It is shown once, store it now:",
        actor.name, actor.id
    );
    println!("{token}");
    Ok(())
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
        Command::Create {
            actor,
            label,
            kind,
            days,
        } => create(&mut graph, &actor, &label, kind, days).await,
        Command::List { actor } => {
            auth::list_tokens(&mut graph, actor.as_deref())
                .await
                .map(|tokens| {
                    if tokens.is_empty() {
                        eprintln!("No tokens.");
                    }
                    for t in tokens {
                        let expires = t.expires_at.map_or("-".to_string(), |e| e.to_string());
                        println!(
                            "{}\t{}\t{}\t{}\t{}",
                            t.id, t.actor_name, t.created_at, expires, t.label
                        );
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
