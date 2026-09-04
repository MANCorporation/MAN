/*!
 * Client application interface for Pluggable Authentication Modules (PAM)
 *
 * PAM is the user authentication system used in many Unix-like operating
 * systems (Linux, NetBSD, MacOS, Solaris, etc.) and handles authentication,
 * account management, session management and passwort changing via pluggable
 * modules. This allows programs (such as `login(1)` and `su(1)`) to
 * authenticate users e.g. in a Kerberos domain as well as locally in
 * `/etc/passwd`.
 *
 * This library provides a safe API to the application-faced parts of PAM,
 * covering multiple use-cases with a high grade of flexibility.
 *
 * It is currently only tested on Linux, but should also support OpenPAM-based
 * platforms like NetBSD. Solaris should also be supported, but the support
 * is untested.
 *
 * # Examples
 *
 * Sample workflow for command line interaction like `su -c`, running a
 * single program as the user after they successfully authenticated:
 *
 * ```no_run
 * use pam_client::{Context, Flag};
 * use pam_client::conv_cli::Conversation; // CLI implementation
 * use std::process::Command;
 * use std::os::unix::process::CommandExt;
 *
 * let mut context = Context::new(
 *    "my-service",       // Service name, decides which policy is used (see `/etc/pam.d`)
 *    None,               // Optional preset user name
 *    Conversation::new() // Handler for user interaction
 * ).expect("Failed to initialize PAM context");
 *
 * // Optionally set some settings
 * context.set_user_prompt(Some("Who art thou? "));
 *
 * // Authenticate the user (ask for password, 2nd-factor token, fingerprint, etc.)
 * context.authenticate(Flag::NONE).expect("Authentication failed");
 *
 * // Validate the account (is not locked, expired, etc.)
 * context.acct_mgmt(Flag::NONE).expect("Account validation failed");
 *
 * // Get resulting user name and map to a user id
 * let username = context.user();
 * let uid = 65535; // Left as an exercise to the reader
 *
 * // Open session and initialize credentials
 * let mut session = context.open_session(Flag::NONE).expect("Session opening failed");
 *
 * // Run a process in the PAM environment
 * let result = Command::new("/usr/bin/some_program")
 *                      .env_clear()
 *                      .envs(session.envlist().iter_tuples())
 *                      .uid(uid)
 *                   // .gid(...)
 *                      .status();
 *
 * // The session is automatically closed when it goes out of scope.
 * ```
 *
 * Sample workflow for non-interactive authentication
 * ```no_run
 * use pam_client::{Context, Flag};
 * use pam_client::conv_mock::Conversation; // Non-interactive implementation
 *
 * let mut context = Context::new(
 *    "my-service",  // Service name
 *    None,
 *    Conversation::with_credentials("username", "password")
 * ).expect("Failed to initialize PAM context");
 *
 * // Authenticate the user
 * context.authenticate(Flag::NONE).expect("Authentication failed");
 *
 * // Validate the account
 * context.acct_mgmt(Flag::NONE).expect("Account validation failed");
 *
 * // ...
 * ```
 *
 * Sample workflow for session management without PAM authentication:
 * ```no_run
 * use pam_client::{Context, Flag};
 * use pam_client::conv_null::Conversation; // Null implementation
 *
 * let mut context = Context::new(
 *    "my-service",     // Service name
 *    Some("username"), // Preset username
 *    Conversation::new()
 * ).expect("Failed to initialize PAM context");
 *
 * // We already authenticated the user by other means (e.g. SSH key) so we
 * // skip `context.authenticate()`.
 *
 * // Validate the account
 * context.acct_mgmt(Flag::NONE).expect("Account validation failed");
 *
 * // Open session and initialize credentials
 * let mut session = context.open_session(Flag::NONE).expect("Session opening failed");
 *
 * // ...
 *
 * // The session is automatically closed when it goes out of scope.
 * ```
 */

/***********************************************************************
 * (c) 2021-2022 Christoph Grenz <christophg+gitorious @ grenz-bonn.de>*
 *                                                                     *
 * This Source Code Form is subject to the terms of the Mozilla Public *
 * License, v. 2.0. If a copy of the MPL was not distributed with this *
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.            *
 ***********************************************************************/

mod c_box;
mod context;
#[cfg(feature = "cli")]
pub mod conv_cli;
pub mod conv_mock;
pub mod conv_null;
mod conversation;
pub mod env_list;
mod error;
mod ffi;
mod resp_buf;
mod session;

#[macro_use]
extern crate bitflags;
use libc::{c_char, c_int};
use std::ffi::CStr;

pub use context::Context;
pub use conversation::ConversationHandler;
pub use error::{Error, ErrorWith};
pub use session::{Session, SessionToken};

use enum_repr::EnumRepr;
use pam_sys::*;

fn char_ptr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
	if ptr.is_null() {
		None
	} else {
		let cstr = unsafe { CStr::from_ptr(ptr) };
		match cstr.to_str() {
			Err(_) => None,
			Ok(s) => Some(s),
		}
	}
}

bitflags! {
	/// Flags for most PAM functions
	#[allow(clippy::upper_case_acronyms)]
	#[repr(transparent)]
	#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
	pub struct Flag: c_int {
		/// Don't generate any messages
		const SILENT = PAM_SILENT as c_int;
		/// Fail with `AUTH_ERROR` if the user has a null authentication token.
		const DISALLOW_NULL_AUTHTOK = PAM_DISALLOW_NULL_AUTHTOK as c_int;
		/// Only update passwords that have aged.
		const CHANGE_EXPIRED_AUTHTOK = PAM_CHANGE_EXPIRED_AUTHTOK as c_int;
		/// Set user credentials.
		#[doc(hidden)]
		const ESTABLISH_CRED = PAM_ESTABLISH_CRED as c_int;
		/// Delete user credentials.
		#[doc(hidden)]
		const DELETE_CRED = PAM_DELETE_CRED as c_int;
		/// Reinitialize user credentials.
		#[doc(hidden)]
		const REINITIALIZE_CRED = PAM_REINITIALIZE_CRED as c_int;
		/// Extend lifetime of user credentials.
		#[doc(hidden)]
		const REFRESH_CRED = PAM_REFRESH_CRED as c_int;
	}
}

#[allow(clippy::upper_case_acronyms)]
impl Flag {
	/// No flags; use default behaviour.
	pub const NONE: Flag = Flag { bits: 0 };
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[EnumRepr(type = "c_int")]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// PAM error codes
pub enum ErrorCode {
	OPEN_ERR = PAM_OPEN_ERR as c_int,
	SYMBOL_ERR = PAM_SYMBOL_ERR as c_int,
	SERVICE_ERR = PAM_SERVICE_ERR as c_int,
	SYSTEM_ERR = PAM_SYSTEM_ERR as c_int,
	BUF_ERR = PAM_BUF_ERR as c_int,
	PERM_DENIED = PAM_PERM_DENIED as c_int,
	AUTH_ERR = PAM_AUTH_ERR as c_int,
	CRED_INSUFFICIENT = PAM_CRED_INSUFFICIENT as c_int,
	AUTHINFO_UNAVAIL = PAM_AUTHINFO_UNAVAIL as c_int,
	USER_UNKNOWN = PAM_USER_UNKNOWN as c_int,
	MAXTRIES = PAM_MAXTRIES as c_int,
	NEW_AUTHTOK_REQD = PAM_NEW_AUTHTOK_REQD as c_int,
	ACCT_EXPIRED = PAM_ACCT_EXPIRED as c_int,
	SESSION_ERR = PAM_SESSION_ERR as c_int,
	CRED_UNAVAIL = PAM_CRED_UNAVAIL as c_int,
	CRED_EXPIRED = PAM_CRED_EXPIRED as c_int,
	CRED_ERR = PAM_CRED_ERR as c_int,
	CONV_ERR = PAM_CONV_ERR as c_int,
	AUTHTOK_ERR = PAM_AUTHTOK_ERR as c_int,
	AUTHTOK_RECOVERY_ERR = PAM_AUTHTOK_RECOVERY_ERR as c_int,
	AUTHTOK_LOCK_BUSY = PAM_AUTHTOK_LOCK_BUSY as c_int,
	AUTHTOK_DISABLE_AGING = PAM_AUTHTOK_DISABLE_AGING as c_int,
	ABORT = PAM_ABORT as c_int,
	AUTHTOK_EXPIRED = PAM_AUTHTOK_EXPIRED as c_int,
	MODULE_UNKNOWN = PAM_MODULE_UNKNOWN as c_int,
	BAD_ITEM = PAM_BAD_ITEM as c_int,
	CONV_AGAIN = PAM_CONV_AGAIN as c_int,
	INCOMPLETE = PAM_INCOMPLETE as c_int,
}

/// Type alias for the result of most PAM methods.
pub type Result<T> = std::result::Result<T, Error>;
/// Type alias for the result of PAM methods that pass back a consumed struct
/// on error.
pub type ExtResult<T, P> = std::result::Result<T, ErrorWith<P>>;

const PAM_SUCCESS: c_int = pam_sys::PAM_SUCCESS as c_int;
