mod ldap;
mod security;

use std::{
    path::Path,
    sync::{Arc, Mutex}, env,
};

use actix_files::NamedFile;
use actix_web::{
    get,
    web::{self, Data},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use anyhow::{anyhow, Result};
use ldap::LdapClient;
use maud::{html, DOCTYPE};
use std::fs::{metadata, read_dir};
use security::RequireApiKey;

struct AppState {
    ldap: Arc<Mutex<LdapClient>>,
}

async fn get_file_or_dir(
    state: Data<AppState>,
    req: HttpRequest,
    uid: String,
    user_root: String,
    filepath: String,
) -> impl Responder {
    let filepath = if filepath == "/" {
        String::new()
    } else {
        filepath
    };
    let mut client = match state.ldap.lock() {
        Ok(client) => client,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };
    let homedir = match client.get_homedir(&uid).await {
        Some(homedir) => homedir,
        None => return HttpResponse::NotFound().finish(),
    };
    let fullpath = Path::new(&homedir).join(user_root).join(filepath.clone());
    match metadata(&fullpath) {
        Ok(doc) => {
            if doc.is_dir() {
                if !filepath.ends_with("/") && filepath.len() > 0 {
                    return HttpResponse::MovedPermanently()
                        .append_header(("Location", format!("/~{}/{}/", uid, filepath)))
                        .finish();
                }
                if metadata(fullpath.join("index.html")).is_ok() {
                    return NamedFile::open(fullpath.join("index.html"))
                        .unwrap()
                        .into_response(&req);
                }
                let files = read_dir(fullpath).unwrap().into_iter().map(|path| {
                    let entry = path.unwrap();
                    if entry.metadata().unwrap().is_dir() {
                        return format!(
                            "{}/",
                            entry
                                .path()
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string()
                        );
                    }
                    entry
                        .path()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string()
                });
                return HttpResponse::Ok().body(
                    html! {
                        (DOCTYPE)
                        html {
                            head {
                                meta charset="utf-8";
                                title { (format!("Index of /{}", filepath)) }
                            }
                            body {
                                h1 { (format!("Index of /{}", filepath)) }
                                hr{}
                                ul {
                                    @for file in files {
                                        li { a href=(format!("{}", file)) {(file)} }
                                    }
                                }
                            }
                        }
                    }
                    .into_string(),
                );
            }
            return NamedFile::open(fullpath).unwrap().into_response(&req);
        }
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    }
}

// #[get("/priv/~{uid}/")]
// async fn get_priv_root(
//     state: Data<AppState>,
//     req: HttpRequest,
//     path: web::Path<(String,)>,
// ) -> impl Responder {
//     let (uid,) = path.into_inner();
//     println!("Priv Root");
//     return get_file_or_dir(state, req, uid, ".html_pages/".to_string(), "".to_string()).await;
// }

#[get("/priv/~{uid}/{filepath:.*}", wrap="RequireApiKey")]
async fn get_priv(
    state: Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (uid, filepath) = path.into_inner();
    println!("Priv");
    return get_file_or_dir(state, req, uid, ".html_pages/".to_string(), filepath).await;
}

// #[get("/~{uid}/")]
// async fn get_pub_root(
//     state: Data<AppState>,
//     req: HttpRequest,
//     path: web::Path<(String,)>,
// ) -> impl Responder {
//     let (uid,) = path.into_inner();
//     return get_file_or_dir(state, req, uid, "public_html/".to_string(), "".to_string()).await;
// }

#[get("/~{uid}/{filepath:.*}", wrap="RequireApiKey")]
async fn get_pub(
    state: Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (uid, filepath) = path.into_inner();
    return get_file_or_dir(state, req, uid, "public_html/".to_string(), filepath).await;
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Arc::from(Mutex::from(
        LdapClient::new(
            &env::var("BIND_DN")?,
            &env::var("BIND_PW")?
        )
        .await,
    ));
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(AppState {
                ldap: client.clone(),
            }))
            .service(get_pub)
            .service(get_priv)
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await
    .map_err(|e| anyhow!("{:?}", e))
}
