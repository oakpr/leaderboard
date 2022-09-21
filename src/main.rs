#[macro_use]
extern crate rocket;
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use rocket::{
	http::Status,
	request::{self, FromRequest, Outcome},
	serde::json::Json,
	Request, State,
};
use rocket_dyn_templates::{context, Template};
use rustbreak::{deser::Ron, PathDatabase};

type Db = PathDatabase<HashMap<String, HashMap<String, u64>>, Ron>;

#[launch]
fn rocket() -> _ {
	rocket::build()
		.manage(
			Db::load_from_path_or_default("./db".parse().unwrap())
				.expect("Failed to init database"),
		)
		.mount("/", routes![home_page, put_score, score_page, css])
		.attach(Template::fairing())
}

#[get("/")]
fn home_page(db: &State<Db>) -> Template {
	db.read(|db| {
		Template::render(
			"index",
			context! {
				games: db.keys().cloned().collect::<Vec<String>>()
			},
		)
	})
	.expect("Failed to read db")
}

#[get("/<game>?<count>")]
fn score_page(game: String, count: Option<usize>, db: &State<Db>) -> Template {
	db.read(|db| {
		let mut scores: Vec<(String, u64)> = if let Some(game) = db.get(&game) {
			game.iter().map(|(a, b)| (a.clone(), *b)).collect()
		} else {
			vec![]
		};
		scores.sort_by(|(_, a), (_, b)| (*b).cmp(a));
		let count = count.unwrap_or(10);
		let more = scores.len() > count;
		let scores: Vec<(String, u64)> = scores.into_iter().take(count).collect();
		Template::render(
			"score",
			context! {
				game: game,
				scores: scores,
				count: count,
				more: more,
			},
		)
	})
	.expect("Failed to read")
}

#[post("/<game>/<player>/<score>?<increment>")]
async fn put_score(
	game: String,
	player: String,
	score: u64,
	db: &State<Db>,
	increment: Option<bool>,
	_auth: Authentication,
) {
	let increment = increment.unwrap_or(false);
	db.write(|db| {
		let stored_score = db.entry(game).or_default().entry(player).or_default();
		if increment {
			*stored_score += score;
		} else {
			*stored_score = score;
		}
	})
	.expect("Failed to insert to db");
	db.save().expect("Failed to write db");
}

#[get("/index.css")]
fn css() -> &'static str {
	include_str!("./index.css")
}

struct Authentication;
#[derive(Debug)]
enum LoginError {
	BadSecret,
	NoSecret,
}

#[async_trait]
impl<'r> FromRequest<'r> for Authentication {
	type Error = LoginError;

	async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
		if let Some(secret) = req.headers().get_one("Authorization") {
			if let Ok(truth) = std::env::var("LEADERBOARD_SECRET") {
				if truth.split(' ').any(|v| v == secret) {
					Outcome::Success(Authentication)
				} else {
					Outcome::Failure((Status::Forbidden, LoginError::BadSecret))
				}
			} else {
				panic!("Failed to get leaderboard secret")
			}
		} else {
			Outcome::Failure((Status::Unauthorized, LoginError::NoSecret))
		}
	}
}