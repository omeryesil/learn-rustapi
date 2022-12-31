//Read more --------------
use actix_web::{get, post, put, delete, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result, ResponseError};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::body::BoxBody;

use serde::{Serialize, Deserialize};

use std::env;
use std::fmt::Display;
use std::sync::Mutex;

// -----------------------------------------------------
// REDIS
// -----------------------------------------------------
use redis::Commands;

fn get_redis_connstr() -> String {
  let redis_host_name = env::var("REDIS_HOSTNAME").expect("missing environment variable REDIS_HOSTNAME");
  let redis_password = env::var("REDIS_PASSWORD").expect("missing environment variable REDIS_PASSWORD");

  let redis_conn_url = format!("rediss://:{}@{}", redis_password, redis_host_name);

  return redis_conn_url;
}


#[get("/simple/{id}")]
pub async fn get_from_redis(
  id: web::Path<String>,
  //redis_pool: web::Data<r2d2::Pool<redis::Client>>
) -> Result<impl Responder> {
  //let mut conn: r2d2::PooledConnection<redis::Client> = redis_pool.get().unwrap();
  // use the connection as usual
  //let val: String = conn.get("oauth2proxyapi-oyesil@cetarisdev.onmicrosoft.com|Acdc11DevOidc").unwrap();
  //println!("{}", val);

  let mut conn = connect();

  let bar: String = redis::cmd("GET")
    .arg("oauth2proxyapi-oyesil@cetarisdev.onmicrosoft.com|Acdc11DevOidc")
    .query(&mut conn)
    .expect("failed to execute GET for 'foo'");


  Ok(web::Json(bar))
}


// ------------------------------------------------------
// TICKET 
// ------------------------------------------------------
#[derive(Serialize, Deserialize)]
struct Ticket{
  id: u32,
  author: String,
}

// Implement Responder Trait for Ticket
impl Responder for Ticket {
  type Body = BoxBody;

  fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
    let res_body = serde_json::to_string(&self).unwrap();

    // Create HttpResponse and set Content Type
    HttpResponse::Ok()
      .content_type(ContentType::json())
      .body(res_body)
  }
}

// ------------------------------------------------------
// ERROR 
// ------------------------------------------------------
#[derive(Debug, Serialize)]
struct ErrNoId {
  id: u32,
  err: String,
}

// Implement ResponseError for ErrNoId
impl ResponseError for ErrNoId {
  fn status_code(&self) -> StatusCode {
      StatusCode::NOT_FOUND
  }

  fn error_response(&self) -> HttpResponse<BoxBody> {
     let body = serde_json::to_string(&self).unwrap();
     let res = HttpResponse::new(self.status_code());
     res.set_body(BoxBody::new(body))
  }
}

// Implement Display for ErrNoId
impl Display for ErrNoId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
     write!(f, "{:?}", self)
  }
}

// -------------------------------------------------------
// AppState 
// -------------------------------------------------------
struct AppState {
  tickets: Mutex<Vec<Ticket>>,
}

// -------------------------------------------------------
// CRUD 
// -------------------------------------------------------

// Create a ticket
#[post("/tickets")]
async fn post_ticket(req: web::Json<Ticket>, data: web::Data<AppState>) -> impl Responder {
   let new_ticket = Ticket {
       id: req.id,
       author: String::from(&req.author),
   };

   let mut tickets = data.tickets.lock().unwrap();

   let response = serde_json::to_string(&new_ticket).unwrap();

   tickets.push(new_ticket);
   HttpResponse::Created()
       .content_type(ContentType::json())
       .body(response)
}

// Get all tickets
#[get("/tickets")]
async fn get_tickets(data: web::Data<AppState>) -> impl Responder {
   let tickets = data.tickets.lock().unwrap();

   let response = serde_json::to_string(&(*tickets)).unwrap();

   HttpResponse::Ok()
       .content_type(ContentType::json())
       .body(response)
}

// Get a ticket with the corresponding id
#[get("/tickets/{id}")]
async fn get_ticket(id: web::Path<u32>, data: web::Data<AppState>) -> Result<Ticket, ErrNoId> {
   let ticket_id: u32 = *id;
   let tickets = data.tickets.lock().unwrap();

   let ticket: Vec<_> = tickets.iter()
                               .filter(|x| x.id == ticket_id)
                               .collect();

   if !ticket.is_empty() {
       Ok(Ticket {
           id: ticket[0].id,
           author: String::from(&ticket[0].author)
       })
   } else {
       let response = ErrNoId {
           id: ticket_id,
           err: String::from("ticket not found")
       };
       Err(response)
   }
}

// Update the ticket with the corresponding id
#[put("/tickets/{id}")]
async fn update_ticket(id: web::Path<u32>, req: web::Json<Ticket>, data: web::Data<AppState>) -> Result<HttpResponse, ErrNoId> {
   let ticket_id: u32 = *id;

   let new_ticket = Ticket {
       id: req.id,
       author: String::from(&req.author),
   };

   let mut tickets = data.tickets.lock().unwrap();

   let id_index = tickets.iter()
                         .position(|x| x.id == ticket_id);

   match id_index {
       Some(id) => {
           let response = serde_json::to_string(&new_ticket).unwrap();
           tickets[id] = new_ticket;
           Ok(HttpResponse::Ok()
               .content_type(ContentType::json())
               .body(response)
           )
       },
       None => {
           let response = ErrNoId {
               id: ticket_id,
               err: String::from("ticket not found")
           };
           Err(response)
       }
   }
}


// Delete the ticket with the corresponding id
#[delete("/tickets/{id}")]
async fn delete_ticket(id: web::Path<u32>, data: web::Data<AppState>) -> Result<Ticket, ErrNoId> {
   let ticket_id: u32 = *id;
   let mut tickets = data.tickets.lock().unwrap();

   let id_index = tickets.iter()
                         .position(|x| x.id == ticket_id);

   match id_index {
       Some(id) => {
           let deleted_ticket = tickets.remove(id);
           Ok(deleted_ticket)
       },
       None => {
           let response = ErrNoId {
               id: ticket_id,
               err: String::from("ticket not found")
           };
           Err(response)
       }
   }
}

// -----------------------------------------------------------------
// MAIN 
// -----------------------------------------------------------------
fn connect() -> redis::Connection {
  //format - host:port
  let redis_host_name =
      env::var("REDIS_HOSTNAME").expect("missing environment variable REDIS_HOSTNAME");

  let redis_password = env::var("REDIS_PASSWORD").unwrap_or_default();

  //if Redis server needs secure connection
  let uri_scheme = match env::var("IS_TLS") {
      Ok(_) => "rediss",
      Err(_) => "redis",
  };

  let redis_conn_url = format!("{}://:{}@{}", uri_scheme, redis_password, redis_host_name);

  redis::Client::open(redis_conn_url)
      .expect("Invalid connection URL")
      .get_connection()
      .expect("failed to connect to Redis")
}



#[actix_web::main]
async fn main() -> std::io::Result<()> {

  // let redis_conn_str : String = get_redis_connstr();

  // let client = redis::Client::open(redis_conn_str).unwrap();
  // let pool: r2d2::Pool<redis::Client> = r2d2::Pool::builder()
  //   .max_size(15)
  //   .build(client)
  //   .unwrap_or_else(|e| panic!("Error building redis pool: {}", e));

  let app_state = web::Data::new(AppState {
                    tickets: Mutex::new(vec![
                        Ticket {
                            id: 1,
                            author: String::from("Jane Doe")
                        },
                        Ticket {
                            id: 2,
                            author: String::from("Patrick Star")
                        }
                    ])
                });

   HttpServer::new(move || {
       App::new()
           .app_data(app_state.clone())
           //.app_data(web::Data::new(pool.clone()))
           .service(get_from_redis)
           .service(post_ticket)
           .service(get_ticket)
           .service(get_tickets)
           .service(update_ticket)
           .service(delete_ticket)
   })
   .bind(("127.0.0.1", 8000))?
   .run()
   .await
}