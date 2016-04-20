extern crate tiny_http;
extern crate rusqlite;
extern crate url;
extern crate core;

use rusqlite::*;

struct Score {
    name: String,
    highscore: i64,
}


fn main() {

    let conn = Connection::open("scores.db").unwrap();

    conn.execute("
        create table if not exists score (
            name    text not null primary key,
            score   int(64) not null
        )",  &[]).unwrap();


    let mut count: i64 = 1;

    let server = tiny_http::Server::http("127.0.0.1:4242").unwrap();
    loop {

        let req : tiny_http::Request = match server.recv() {
            Ok(rq) => rq,
            Err(_) => { println!("Error"); break; }
        };

        match *req.method() {
            tiny_http::Method::Get => handle_get(req, &conn, count),
            tiny_http::Method::Post => {if handle_post(req, &conn) {
                if count > (count + 1) {
                    // Overflow.. ^^
                    count = 1;
                } else {
                    count += 1;
                }
            }},
            _ => {}
        }

    }

}


fn handle_get(req: tiny_http::Request, conn: &Connection, count: i64) {

    let mut url: String = req.url().to_string();
    url.remove(0);

    let local_count: i64 = url.parse().unwrap_or(-1);
    if local_count < 0 {
        return;
    }
    if local_count == count {
        let response = tiny_http::Response::empty(tiny_http::StatusCode::from(418));
        match req.respond(response)  {
            Ok(_) => {},
            _     => println!("Something wrong with handle_get()")
        };
        return;
    }

    // answer with the new scoreboard

    let scores = get_top_five(conn);



    let mut data: String = "{\"cache\": ".to_string();
    data.push_str(&count.to_string());
    data.push_str(", \"scores\": {");
    let mut i = 0;
    let k = scores.len();
    for score in scores {
        data.push_str("\"");
        data.push_str(&score.name);
        data.push_str("\": ");
        data.push_str(&score.highscore.to_string());
        if i < k - 1 {
            data.push_str(",");
        }
        i += 1;
    }
    data.push_str("}}");
    let response = tiny_http::Response::from_string(data);
    match req.respond(response) {
        Ok(_) => {},
        _     => println!("Something wrong with get()")
    };
}

fn handle_post(req: tiny_http::Request, conn: &Connection) -> bool {
    let mut req = req;
    let mut content = String::new();
    match req.as_reader().read_to_string(&mut content) {
        Ok(_) => {},
        _     => return false
    };
    let mut toks: core::str::Split<&str> = content.split(",");
    let name = toks.next().unwrap_or("Anon");
    let score: i64 = toks.next().unwrap_or("nope").to_string().parse().unwrap_or(-1);

    if score < 0 {
        return false;
    }

    let s = Score {
        name: name.to_string(),
        highscore: score
    };

    add_score(conn, s);

    true
}







fn add_score(conn: &Connection, score: Score) {

    let mut stmt = conn.prepare("select score from score where name = (:name)").unwrap();
    let rows: rusqlite::Rows = stmt.query_named(&[(":name", &score.name)]).unwrap();
    if rows.count() != 0 {
        update(conn, score);
    } else {
        insert(conn, score);
    }

}

fn update(conn: &Connection, score: Score) {
    let mut stmt = conn.prepare("
        update score
        set score = (:score)
        where name = (:name) and score < (:score)
    ").unwrap();
    stmt.execute_named(&[(":score", &score.highscore), (":name", &score.name)]).unwrap();
}

fn insert(conn: &Connection, score: Score) {
    let mut stmt = conn.prepare("
        insert into score
        values (:name, :score)
    ").unwrap();
    stmt.execute_named(&[(":score", &score.highscore), (":name", &score.name)]).unwrap();
}

fn get_top_five(conn: &Connection) -> Vec<Score> {
    let mut scores = vec![];
    let mut stmt = conn.prepare("select name, score from score order by score desc limit 5").unwrap();
    let iter = stmt.query_map(&[], |row| {
        Score {
            name: row.get(0),
            highscore: row.get(1)
        }
    }).unwrap();
    for score in iter {
        scores.push(score.unwrap());
    }
    scores
}
