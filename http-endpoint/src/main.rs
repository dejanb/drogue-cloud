mod basic_auth;
mod ttn;
mod command;

use actix_web::{
    get, http::header, middleware, post, put, web, App, HttpResponse, HttpServer, Responder,
};

use drogue_cloud_endpoint_common::downstream::{
    DownstreamSender, Publish, Outcome, PublishResponse,
};

use drogue_cloud_endpoint_common::error::HttpEndpointError;
use serde::Deserialize;
use serde_json::json;

use actix_web_httpauth::middleware::HttpAuthentication;
use dotenv::dotenv;
use envconfig::Envconfig;

use self::basic_auth::basic_validator;
use actix_web::middleware::Condition;
use command::{CommandMessage, CommandRouter};
use actix::SystemService;
use actix::Actor;
use actix::prelude::*;
use std::{thread, time};

use actix_web_actors::HttpContext;
use actix_web::body::Body;
use actix_web::http::{StatusCode};

use crate::command::{CommandHandler, CommandSubscribe, CommandUnsubscribe, TestHandler};

#[derive(Envconfig, Clone, Debug)]
struct Config {
    #[envconfig(from = "MAX_JSON_PAYLOAD_SIZE", default = "65536")]
    pub max_json_payload_size: usize,
    #[envconfig(from = "BIND_ADDR", default = "127.0.0.1:8080")]
    pub bind_addr: String,
    #[envconfig(from = "ENABLE_AUTH", default = "false")]
    pub enable_auth: bool,
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().json(json!({"success": true}))
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[derive(Deserialize)]
pub struct PublishOptions {
    model_id: Option<String>,
}

#[post("/publish/{device_id}/{channel}")]
async fn publish(
    endpoint: web::Data<DownstreamSender>,
    web::Path((device_id, channel)): web::Path<(String, String)>,
    web::Query(opts): web::Query<PublishOptions>,
    req: web::HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, HttpEndpointError> {
    log::info!("Published to '{}'", channel);

    match endpoint
        .publish(
            Publish {
                channel,
                device_id,
                model_id: opts.model_id,
                content_type: req
                    .headers()
                    .get(header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string()),
            },
            body,
        )
        .await
    {
        // ok, and accepted
        Ok(PublishResponse {
            outcome: Outcome::Accepted,
        }) => {
            let mut response = HttpResponse::Accepted();

            log::info!("Accepted");

            let handler = CommandHandler;
            let context = HttpContext::create(handler);
            Ok(HttpResponse::Ok().streaming(context))

//            Ok(HttpResponse::build(StatusCode::OK)
//                .body(Body::Message(Box::new(context))))


//            let handler = TestHandler;
//            let device = handler.start();
//            let sub = CommandSubscribe("test".to_string(), device.recipient());
//            CommandRouter::from_registry()
//                .send(sub)
//                .await;
//
//
//            thread::sleep(time::Duration::from_secs(5));
//
//            CommandRouter::from_registry()
//                .send(CommandUnsubscribe("test".to_string()))
//                .await;
//
//            Ok(response.finish())


        },

        // ok, but rejected
        Ok(PublishResponse {
            outcome: Outcome::Rejected,
        }) => Ok(HttpResponse::NotAcceptable().finish()),

        // internal error
        Err(err) => Ok(HttpResponse::InternalServerError()
            .content_type("text/plain")
            .body(err.to_string())),
    }


//    let result = endpoint.publish_http(
//        Publish {
//            channel,
//            device_id,
//            model_id: opts.model_id,
//            content_type: req
//                .headers()
//                .get(header::CONTENT_TYPE)
//                .and_then(|v| v.to_str().ok())
//                .map(|s| s.to_string()),
//        },
//        body,
//    ).await;
//
//    let device = CommandHandler(HttpResponse::Accepted()).start();
//    let sub = CommandSubscribe("test".to_string(), device.recipient());
//    CommandRouter::from_registry()
//        .send(sub)
//        .await;
//
//    thread::sleep(time::Duration::from_secs(5));
//
//    CommandRouter::from_registry()
//        .send(CommandUnsubscribe("test".to_string()))
//        .await;
//
//
//
//    result
}


#[post("/test")]
async fn test(
    endpoint: web::Data<DownstreamSender>,
    web::Query(opts): web::Query<PublishOptions>,
    req: web::HttpRequest,
    body: web::Bytes,
) -> impl Responder {
    log::info!("Test");


    let handler = CommandHandler;
    let context = HttpContext::create(handler);

    HttpResponse::Ok().streaming(context)

}

#[put("/telemetry/{tenant}/{device}")]
async fn telemetry(
    endpoint: web::Data<DownstreamSender>,
    web::Path((tenant, device)): web::Path<(String, String)>,
    req: web::HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, HttpEndpointError> {
    log::info!(
        "Sending telemetry for device '{}' belonging to tenant '{}'",
        device,
        tenant
    );
    endpoint.publish_http(
        Publish {
            channel: tenant,
            device_id: device,
            model_id: None,
            content_type: req
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
        },
        body,
    ).await
}

#[post("/command-service")]
async fn command_service(
    req: web::HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, actix_web::Error> {

    log::info!("Received command");

    let command_msg = CommandMessage(
        String::from_utf8(body.as_ref().to_vec()).unwrap(),
    );

    CommandRouter::from_registry()
        .send(command_msg)
        .await;

    HttpResponse::Ok().await

}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    dotenv().ok();

    log::info!("Starting HTTP service endpoint z");

    let sender = DownstreamSender::new()?;

    let config = Config::init_from_env()?;
    let enable_auth = config.enable_auth;
    let max_json_payload_size = config.max_json_payload_size;

    HttpServer::new(move || {
        //let jwt_auth = HttpAuthentication::bearer(jwt_validator);
        let basic_auth = HttpAuthentication::basic(basic_validator);

        App::new()
            .wrap(Condition::new(enable_auth, basic_auth))
            .wrap(middleware::Logger::default())
            .data(web::JsonConfig::default().limit(max_json_payload_size))
            .data(sender.clone())
            .service(index)
            .service(publish)
            .service(telemetry)
            .service(test)
            .service(ttn::publish)
            .service(command_service)
    })
    .bind(config.bind_addr)?
    .run()
    .await?;

    Ok(())
}
