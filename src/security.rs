use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    HttpResponse,
};
use futures::future::LocalBoxFuture;
use lazy_static::lazy_static;
use std::{
    env,
    future::{self, Ready},
};

const API_KEY_NAME: &str = "api_key";

lazy_static! {
    static ref API_KEY: String = env::var("API_KEY").unwrap();
}

pub struct RequireApiKey;

impl<S> Transform<S, ServiceRequest> for RequireApiKey
where
    S: Service<
        ServiceRequest,
        Response = ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
{
    type Response = ServiceResponse<actix_web::body::BoxBody>;
    type Error = actix_web::Error;
    type Transform = ApiKeyMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ready(Ok(ApiKeyMiddleware {
            service,
            log_only: false,
        }))
    }
}

pub struct ApiKeyMiddleware<S> {
    service: S,
    log_only: bool,
}

impl<S> Service<ServiceRequest> for ApiKeyMiddleware<S>
where
    S: Service<
        ServiceRequest,
        Response = ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
{
    type Response = ServiceResponse<actix_web::body::BoxBody>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, actix_web::Error>>;

    fn poll_ready(
        &self,
        ctx: &mut core::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let response = |req: ServiceRequest, response: HttpResponse| -> Self::Future {
            Box::pin(async { Ok(req.into_response(response)) })
        };

        match req.headers().get(API_KEY_NAME) {
            Some(key) if key != &API_KEY.to_string() => {
                if self.log_only {
                    println!("Incorrect api api provided!!!")
                } else {
                    return response(req, HttpResponse::Unauthorized().body("incorrect api key"));
                }
            }
            None => {
                if self.log_only {
                    println!("Missing api key!!!")
                } else {
                    return response(req, HttpResponse::Unauthorized().body("missing api key"));
                }
            }
            _ => (), // just passthrough
        }

        if self.log_only {
            println!("Performing operation")
        }

        let future = self.service.call(req);

        Box::pin(async move {
            let response = future.await?;

            Ok(response)
        })
    }
}