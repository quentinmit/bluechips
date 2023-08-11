use rocket::http::{Cookie, CookieJar, Status};
use rocket::State;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::{Serialize, Deserialize};
use rocket::outcome::try_outcome;
use sea_orm::DatabaseConnection;
use serde_json::{json, from_str};
use password_auth::{verify_password, VerifyError};
use std::time::SystemTime;

mod session;
pub use session::SessionManager;

use rand::random;
pub fn rand_string(size: usize) -> String {
    (0..)
        .map(|_| random::<char>())
        .filter(|c| c.is_ascii())
        .map(char::from)
        .take(size)
        .collect()
}

pub(crate) fn now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("time is after epoch")
        .as_secs() as i64
}

#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// This error occurs when attempting to create a user with an invalid email address.
    #[error("That is not a valid email address.")]
    InvalidEmailAddressError,

    /// Thrown when the requested user does not exists.
    #[error("Could not find any user that fits the specified requirements.")]
    UserNotFoundError,

    /// This error is thrown when trying to retrieve `Users` but it isn't being managed by the app.
    /// It can be fixed adding `.manage(users)` to the app, where `users` is of type `Users`.
    #[error("UnmanagedStateError: failed retrieving `Users`. You may be missing `.manage(users)` in your app.")]
    UnmanagedStateError,

    #[error("UnauthenticatedError: The operation failed because the client is not authenticated.")]
    UnauthenticatedError,
    /// This error occurs when a user tries to log in, but their account doesn't exists.
    #[error("The email \"{0}\" is not registered. Try signing up first.")]
    EmailDoesNotExist(String),
    /// This error is thrown when a user tries to sign up with an email that already exists.
    #[error("That email address already exists. Try logging in.")]
    EmailAlreadyExists,
    /// This error occurs when the user does exists, but their password was incorrect.
    #[error("Incorrect email or password")]
    UnauthorizedError,
    #[error("Incorrect password: {0}")]
    VerifyError(#[from] VerifyError),

    // /// A wrapper around [`validator::ValidationError`].
    // #[error("{0}")]
    // FormValidationError(#[from] validator::ValidationError),

    // /// A wrapper around [`validator::ValidationErrors`].
    // #[error("FormValidationErrors: {0}")]
    // FormValidationErrors(#[from] validator::ValidationErrors),

    #[error("DbErr: {0}")]
    DbErr(#[from] sea_orm::DbErr),

    //#[error("SqlxError: {0}")]
    //SqlxError(#[from] sqlx::Error),

    /// A wrapper around [`serde_json::Error`].
    #[error("SerdeError: {0}")]
    SerdeError(#[from] serde_json::Error),
}

impl From<&Error> for Status {
    fn from(value: &Error) -> Self {
        match *value {
            Error::UnauthorizedError => Status::Unauthorized,
            Error::UnauthenticatedError => Status::Unauthorized,
            Error::UserNotFoundError => Status::Unauthorized,
            Error::VerifyError(VerifyError::PasswordInvalid) => Status::Unauthorized,
            _ => Status::InternalServerError,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// The Session guard can be used to retrieve user session data.
/// Unlike `User`, using session does not verify that the session data is
/// still valid. Since the client could have logged out, or their session
/// may have expired. The Session guard is intended for purposes where
/// verifying the validity of the session data is unnecessary.
///
/// Note that,
/// session data is already captured by the [`Auth`](`crate::Auth`) guard and stored in the public [`session`](`crate::Auth`) field.
/// So it is not necessary to use them together.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Session {
    /// It represents the Unix time in which the user logged in. It is measured in seconds.
    pub time_stamp: i64,
    /// The user id as it is stored on the database.
    pub id: i32,
    pub username: String,
    /// A random authentication token key.
    pub auth_key: String,
}


#[async_trait]
impl<'r> FromRequest<'r> for Session {
    type Error = Error;
    async fn from_request(request: &'r Request<'_>) -> Outcome<Session, Self::Error> {
        let cookies = request.cookies();

        if let Some(session) = get_session(cookies) {
            Outcome::Success(session)
        } else {
            Outcome::Failure((Status::Unauthorized, Error::UnauthorizedError))
        }
    }
}

fn get_session(cookies: &CookieJar) -> Option<Session> {
    let session = cookies.get_private("rocket_auth")?;
    from_str(session.value()).ok()
}


pub struct Auth<'a> {
    /// `Auth` includes in its fields a [`Users`] instance. Therefore, it is not necessary to retrieve `Users` when using this guard.
    pub users: Users<'a>,
    pub cookies: &'a CookieJar<'a>,
    pub session: Option<Session>,
}

#[derive(FromForm, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct Login {
    pub username: String,
    pub(crate) password: String,
}

impl<'a> Auth<'a> {
    pub async fn login(&self, form: &Login, db: &DatabaseConnection) -> Result<()> {
        let form_pwd = &form.password.as_bytes();
        let user = Query::find_user_by_username(db, &form.username).await?.ok_or(Error::UserNotFoundError)?;
        let user_pwd = &user.password.ok_or(Error::UnauthorizedError)?;
        verify_password(form_pwd, user_pwd)?;
        let key = self.set_auth_key(user.id).await?;
        let session = Session {
            id: user.id,
            username: user.username,
            auth_key: key,
            time_stamp: now(),
        };
        let to_str = format!("{}", json!(session));
        self.cookies.add_private(Cookie::new("rocket_auth", to_str));
        Ok(())
    }

    async fn set_auth_key(&self, user_id: i32) -> Result<String> {
        let key = rand_string(15);
        self.users.sess.insert(user_id, key.clone()).await?;
        Ok(key)
    }

    pub async fn is_auth(&self) -> bool {
        if let Some(session) = &self.session {
            self.users.sess.get(session.id).await.map(|auth_key| auth_key == session.auth_key).unwrap_or_default()
        } else {
            false
        }
    }

    pub async fn get_user(&self, db: &DatabaseConnection) -> Option<User> {
        if !self.is_auth().await {
            return None;
        }
        let id = self.session.as_ref()?.id;
        Query::get_user_by_id(db, id).await.unwrap_or_default()
    }
}

pub use crate::entities::user::Model as User;
use crate::service::*;
#[derive(PartialEq, Eq, Clone, Hash)]
pub struct Resident(User);

pub struct Users<'a> {
    sess: &'a State<Box<dyn SessionManager>>,
}

impl<'a> Users<'a> {
    
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Users<'r> {
    type Error = Error;
    async fn from_request(req: &'r Request<'_>) -> Outcome<Users<'r>, Error> {
        let session_manager: &State<Box<dyn SessionManager>> = if let Outcome::Success(session_manager) = req.guard().await {
            session_manager
        } else {
            return Outcome::Failure((Status::InternalServerError, Error::UnmanagedStateError));
        };

        Outcome::Success(Users {
            sess: session_manager,
        })
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Auth<'r> {
    type Error = Error;
    async fn from_request(req: &'r Request<'_>) -> Outcome<Auth<'r>, Error> {
        let session: Option<Session> = if let Outcome::Success(session) = req.guard().await {
            Some(session)
        } else {
            None
        };

        let users: Users = if let Outcome::Success(users) = req.guard().await {
            users
        } else {
            return Outcome::Failure((Status::InternalServerError, Error::UnmanagedStateError));
        };

        Outcome::Success(Auth {
            users,
            session,
            cookies: req.cookies(),
        })
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = Error;
    async fn from_request(request: &'r Request<'_>) -> Outcome<User, Error> {
        let auth: Auth = try_outcome!(request.guard().await);
        let db: &State<DatabaseConnection> = match request.guard().await {
            Outcome::Success(db) => db,
            _ => return Outcome::Failure((Status::InternalServerError, Error::UnmanagedStateError)),
        };
        let db: &DatabaseConnection = db as &DatabaseConnection;
        if let Some(user) = auth.get_user(db).await {
            Outcome::Success(user)
        } else {
            Outcome::Failure((Status::Unauthorized, Error::UnauthorizedError))
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Resident {
    type Error = Error;
    async fn from_request(request: &'r Request<'_>) -> Outcome<Resident, Error> {
        let user: User = try_outcome!(request.guard().await);
        if user.resident {
            Outcome::Success(Resident(user))
        } else {
            Outcome::Failure((Status::Forbidden, Error::UnauthorizedError))
        }
    }
}
