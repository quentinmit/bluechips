#[macro_use] extern crate rocket;
use rocket::fs::FileServer;
use rocket::response::{Flash, Redirect};
use rocket::request::FlashMessage;
use rocket::State;
use askama::Template; // bring trait in scope
use sea_orm::Database;
use sea_orm::sea_query::IntoCondition;

mod entities;
use entities::{prelude::*, *};
use sea_orm::{prelude::*, *};

#[derive(Template)] // this will generate the code...
#[template(path = "status/index.html")] // using the template in this path, relative
// to the `templates` dir in the crate root
struct StatusTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
    expenditures: Vec<ExpenditureDisplay>,
}

#[derive(FromQueryResult)]
struct ExpenditureDisplay {
    id: i32,
    amount: Currency,
    mine: bool,
    description: Option<String>,
    date: Option<Date>,
    spender_name: String,
    share_amount: Currency,
}

#[get("/")]
async fn status<'a>(db: &State<DatabaseConnection>, flash: Option<FlashMessage<'a>>) -> StatusTemplate<'a> {
    let db = db as &DatabaseConnection;
    let user_id = 1;
    let expenditures = Expenditure::find()
        // TODO: spender == user or any (split where user == user and share != 0)
        //.filter(expenditure::Column::SpenderId.eq(1))
        .order_by_desc(expenditure::Column::Date)
        .limit(10)
        .select_only()
        .columns([expenditure::Column::Id, expenditure::Column::Amount, expenditure::Column::Description, expenditure::Column::Date])
        .join(JoinType::InnerJoin, expenditure::Relation::Spender.def())
        .column_as(user::Column::Id.eq(user_id), "mine")
        .column_as(user::Column::Name, "spender_name")
        .join(JoinType::LeftJoin, expenditure::Relation::Split.def().on_condition(move |_, right| {Expr::col((right, split::Column::UserId)).eq(user_id).into_condition()}))
        .column_as(split::Column::Share, "share_amount")
        .into_model::<ExpenditureDisplay>()
        .all(db)
        .await.unwrap();
    StatusTemplate{title: None, flash: flash, mobile_client: false, expenditures}
}

#[get("/spend")]
fn spend_index() -> Option<()> {
    None
}
#[get("/spend/<id>/edit")]
fn spend_edit(id: i32) -> Option<()> {
    None
}
#[get("/spend/<id>/delete")]
fn spend_delete(id: i32) -> Option<()> {
    None
}

#[get("/transfer")]
fn transfer() -> Option<()> {
    None
}

#[get("/history")]
fn history_index() -> Option<()> {
    None
}

#[get("/user")]
fn user_index() -> Option<()> {
    None
}

#[launch]
async fn rocket() -> _ {
    let db = Database::connect("sqlite://database.sqlite3").await.unwrap();
    rocket::build()
        .manage(db)
        .mount("/", routes![status])
        .mount("/js", FileServer::from("public/js/"))
        .mount("/css", FileServer::from("public/css/"))
        .mount("/icons", FileServer::from("public/icons/"))
}
