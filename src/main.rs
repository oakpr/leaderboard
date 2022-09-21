#![allow(clippy::let_unit_value)]
#![allow(clippy::needless_pass_by_value)]
#![warn(clippy::pedantic)]
#![allow(clippy::no_effect_underscore_binding)]
#[macro_use]
extern crate rocket;
use std::collections::HashMap;

use rocket::{
	http::Status,
	request::{self, FromRequest, Outcome},
	serde::json::Json,
	Request, State,
};
use rocket_dyn_templates::{context, Template};
use rustbreak::{deser::Ron, PathDatabase};
type Leaderboard = HashMap<String, HashMap<String, u64>>;
type Db = PathDatabase<Leaderboard, Ron>;

#[launch]
fn rocket() -> _ {
	rocket::build()
		.manage(
			Db::load_from_path_or_default("./db".parse().unwrap())
				.expect("Failed to init database"),
		)
		.mount(
			"/",
			routes![home_page, put_score, score_page, css, get_board],
		)
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
#[allow(clippy::needless_pass_by_value)]
fn put_score(
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

#[get("/<game>/board.json?<top>")]
fn get_board(game: &str, db: &State<Db>, top: Option<usize>) -> Json<HashMap<String, u64>> {
	db.read(|db| {
		if let Some(game) = db.get(game) {
			if let Some(top) = top {
				let mut scores: Vec<(String, u64)> = game.clone().into_iter().collect();
				scores.sort_by(|(_, a), (_, b)| (*b).cmp(a));
				let scores = scores.into_iter().take(top).collect();
				Json(scores)
			} else {
				Json(game.clone())
			}
		} else {
			Json(HashMap::new())
		}
	})
	.expect("Failed to read db")
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
