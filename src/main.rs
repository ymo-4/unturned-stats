use std::{cell::RefCell, collections::HashMap, net::Ipv4Addr, rc::Rc, thread, time};
use steamworks::{Client, PlayerDetailsCallbacks, ServerListCallbacks};

const APP_ID: u32 = 304930;

struct Data {
    total_players: i32,
    total_time: f32,
    max_players: (i32, Ipv4Addr, u16, String),
    max_time: (f32, String, Ipv4Addr, u16, String),
    total_servers: i32,
    ping_failed: i32,

    details_completed: bool,
    refresh_complete: bool,
}

fn main() {
    let (client, single) = Client::init_app(APP_ID).unwrap();

    let data = Rc::new(RefCell::new(Data {
        total_players: 0,
        total_time: 0f32,
        max_players: (0, Ipv4Addr::UNSPECIFIED, 0, String::new()),
        max_time: (0f32, String::new(), Ipv4Addr::UNSPECIFIED, 0, String::new()),
        total_servers: 0,
        ping_failed: 0,

        details_completed: false,
        refresh_complete: false,
    }));
    let responded = Rc::clone(&data);
    let failed = Rc::clone(&data);
    let complete = Rc::clone(&data);

    let filters = HashMap::new();
    let start = time::Instant::now();
    client.matchmaking_servers().internet_server_list(APP_ID, &filters, ServerListCallbacks::new(
        Box::new(move |req, id| {
            let details = {
                let mut data = responded.borrow_mut();

                let details = req.lock().unwrap().get_server_details(id).unwrap();
                let players = details.players;

                data.total_players += players;
                if data.max_players.0 < players {
                    data.max_players = (players, details.addr, details.query_port, details.server_name.clone());
                }

                details
            };

            let add_player = Rc::clone(&responded);
            let complete = Rc::clone(&responded);
            responded.borrow_mut().details_completed = false;
            client.matchmaking_servers().player_details(details.addr, details.query_port, PlayerDetailsCallbacks::new(
                Box::new(move |name, _score, playedtime| {
                    let mut data = add_player.borrow_mut();

                    data.total_time += playedtime;
                    if data.max_time.0 < playedtime {
                        data.max_time = (
                            playedtime,
                            name.to_string_lossy().to_string(),
                            details.addr,
                            details.query_port,
                            details.server_name.clone()
                        );
                    }
                }),
                Box::new(|| {

                }),
                Box::new(move || {
                    complete.borrow_mut().details_completed = true;
                })
            ));
        }),
        Box::new(move |_req, _id| {
            failed.borrow_mut().ping_failed += 1;
        }),
        Box::new(move |req, _res| {
            let mut req = req.lock().unwrap();

            let mut data = complete.borrow_mut();
            data.refresh_complete = true;
            data.total_servers = req.get_server_count().unwrap();

            req.release();
        })
    )).unwrap();

    loop {
        single.run_callbacks();

        let data = data.borrow_mut();
        if data.refresh_complete && data.details_completed {
            let elapsed = start.elapsed();
            println!("complete. took {:?}", elapsed);

            println!(
                "servers: {}\ntotal online: {}\ntotal time: {}\nmax online: {:?}\nmax time: {:?}\nfailed to ping: {}",
                data.total_servers,
                data.total_players,
                data.total_time,
                data.max_players,
                data.max_time,
                data.ping_failed,
            );

            break;
        }

        thread::sleep(time::Duration::from_millis(2));
    }
}
