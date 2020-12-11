use actix::prelude::*;
use actix_broker::BrokerSubscribe;

use std::collections::HashMap;

use actix_web_actors::HttpContext;
use actix_web::web::Bytes;

use std::{thread, time};

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub struct CommandMessage(pub String);

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub struct CommandSubscribe(pub String, pub Device);

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub struct CommandUnsubscribe(pub String);



type Device = Recipient<CommandMessage>;
//type Room = HashMap<String, Client>;

#[derive(Default)]
pub struct CommandRouter {
    pub devices: HashMap<String, Device>,
}

impl CommandRouter {

    fn subscribe(&mut self, id: String, device: Device) {

        log::info!("Sub {}", id);

        self.devices.insert(id, device);
        
    }

    fn unsubscribe(&mut self, id: String) {
        
        log::info!("Unsub");

        self.devices.remove(&id);

    }

}

impl Actor for CommandRouter {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_system_async::<CommandMessage>(ctx);
    }
}

impl Handler<CommandMessage> for CommandRouter {
    type Result = ();

    fn handle(&mut self, msg: CommandMessage, _ctx: &mut Self::Context) -> Self::Result {
        log::info!("Got it");

        for (id, device) in self.devices.drain() {
            log::info!("Sending it to device {}", id);
            device.do_send(msg.to_owned());
            log::info!("Sent {}", id);
        }
    }
}


impl Handler<CommandSubscribe> for CommandRouter {
    type Result = ();

    fn handle(&mut self, msg: CommandSubscribe, _ctx: &mut Self::Context) {

        let CommandSubscribe(id, device) = msg;

        self.subscribe(id, device);


    }

}

impl Handler<CommandUnsubscribe> for CommandRouter {
    type Result = ();

    fn handle(&mut self, msg: CommandUnsubscribe, _ctx: &mut Self::Context) {

        self.unsubscribe(msg.0);


    }

}

impl SystemService for CommandRouter {}
impl Supervised for CommandRouter {}


pub struct CommandHandler;

impl Actor for CommandHandler {
    type Context = HttpContext<Self>;

    fn started(&mut self, ctx: &mut HttpContext<Self>) {
       log::info!("Actor is alive");
        let sub = CommandSubscribe("test".to_string(), ctx.address().recipient());
        CommandRouter::from_registry()
            .send(sub)
            .into_actor(self)
            .then(|id, act, _ctx| {
                log::info!("Sent sub!");
                fut::ready(())
            })
            .wait(ctx);
        //ctx.run_later(time::Duration::from_millis(5000), |slf, ctx| slf.write(ctx));
        ctx.run_later(time::Duration::from_millis(5000), |slf, ctx| ctx.write_eof());
    }

    fn stopped(&mut self, ctx: &mut HttpContext<Self>) {
        log::info!("Actor is stopped");
        CommandRouter::from_registry()
            .send(CommandUnsubscribe("test".to_string()))
            .into_actor(self)
            .then(|id, act, _ctx| {
                log::info!("Sent unsub!");
                fut::ready(())
            })
            .wait(ctx);
    }
}

impl Handler<CommandMessage> for CommandHandler {
    type Result = ();

    fn handle(&mut self, msg: CommandMessage, ctx: &mut HttpContext<Self>) {
        log::info!("I gotz the message");
        ctx.write(Bytes::from(msg.0));
        ctx.write_eof()
    }
    
}

pub struct TestHandler;

impl Actor for TestHandler {
    type Context = Context < Self >;

    fn started( & mut self, ctx: & mut Self::Context) {
        log::info!("Test actor is alive");
        let sub = CommandSubscribe("test".to_string(), ctx.address().recipient());
        CommandRouter::from_registry()
            .send(sub)
            .into_actor(self)
            .then(|id, act, _ctx| {
                log::info!("Test sent sub!");
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopped( & mut self, ctx: & mut Self::Context) {
        log::info!("Test actor stopped");
    }
}

impl Handler<CommandMessage> for TestHandler {
    type Result = ();

    fn handle(&mut self, msg: CommandMessage, ctx: &mut Context<Self>) {
        log::info!("Test gotz the message");
    }

}
