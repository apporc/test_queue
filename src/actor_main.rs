use log::info;

// 示例消息：Stop
#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct Stop;

// 示例消息：Compute
#[derive(Message, Debug, Clone)]
#[rtype(result = "i32")]
pub struct Compute {
    value: i32,
}

// 示例 Actor：Executor
struct Executor {
    count: i32, // 示例状态
}

impl Executor {
    fn new() -> Self {
        Executor { count: 0 }
    }
}

impl Actor for Executor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Executor started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("Executor stopped");
    }
}

impl Handler<Stop> for Executor {
    type Result = ();

    fn handle(&mut self, _msg: Stop, _ctx: &mut Self::Context) -> Self::Result {
        info!("Executor received stop signal");
        // 这里可以停止 Actor，但同步模型中需要外部控制
    }
}

impl Handler<Compute> for Executor {
    type Result = i32;

    fn handle(&mut self, msg: Compute, _ctx: &mut Self::Context) -> Self::Result {
        self.count += 1;
        info!("Executor computing: {} (count: {})", msg.value, self.count);
        msg.value * 2
    }
}

// 启动 Actor
fn spawn_actor<A: Actor>(actor: A) -> Addr<A> {
    let (tx, rx) = sync_channel(1024);
    let ctx = Context::new(tx.clone());
    let addr = Addr { tx };

    thread::spawn(move || {
        let mut actor = actor;
        let mut ctx = ctx;
        actor.started(&mut ctx);

        while let Ok(msg) = rx.recv() {
            msg.handle(&mut actor, &mut ctx);
        }

        actor.stopped(&mut ctx);
    });

    addr
}

fn main() {
    let executor = Executor::new();
    let addr = spawn_actor(executor);

    // 发送消息
    addr.do_send(Compute { value: 5 });
    addr.do_send(Compute { value: 10 });
    addr.do_send(Stop);

    // 等待 Actor 处理（仅演示）
    thread::sleep(std::time::Duration::from_secs(1));
}
