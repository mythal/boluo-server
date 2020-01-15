pub struct Validator<'a, T: ?Sized>(&'a [(&'static str, &'a (dyn Fn(&T) -> bool + Sync))]);

impl<'a, T: ?Sized> Validator<'a, T> {
    pub fn run<U: AsRef<T>>(&self, value: U) -> Result<(), &'static str> {
        let Validator(sub_validators) = *self;
        for (message, validator) in sub_validators {
            if !validator(value.as_ref()) {
                return Err(message);
            }
        }
        Ok(())
    }
}

macro_rules! min {
    ($n: expr) => {
        |s| s.len() >= $n
    };
}

macro_rules! max {
    ($n: expr) => {
        |s| s.len() <= $n
    };
}

macro_rules! is_match {
    ($pattern: expr) => {
        |s| regex!($pattern).is_match(&*s)
    };
}

pub static PASSWORD: Validator<str> = Validator(&[
    ("Password length shall not be less than 8.", &min!(8)),
    ("Password length shall not be more than 128.", &max!(128)),
]);

pub static USERNAME: Validator<str> = Validator(&[
    ("Username length shall not be less than 3.", &min!(3)),
    ("Username length shall not be more than 32.", &max!(32)),
    (
        r#"Username can only contain letters, "_" and numbers."#,
        &is_match!(r#"^[\w_\d]+$"#),
    ),
]);

pub static NICKNAME: Validator<str> = Validator(&[
    ("Nickname length shall not be less than 2.", &min!(2)),
    ("Username length shall not be more than 32.", &max!(32)),
]);

pub static EMAIL: Validator<str> = Validator(&[
    ("E-mail address length shall not be less than 5.", &min!(5)),
    ("E-mail address length shall not be more than 254.", &max!(254)),
    // How to validate an email address using a regular expression?
    // https://stackoverflow.com/q/201323
    ("Invalid e-mail address", &is_match!(r"^\S+@\S+\.\S+$")),
]);

pub static BIO: Validator<str> = Validator(&[("Bio shall not be more than 2048.", &max!(2048))]);

#[test]
fn validator_test() {
    assert_eq!(PASSWORD.run("whoa!whoa!".to_string()), Ok(()));
    assert!(PASSWORD.run("whoa!").is_err());

    assert_eq!(USERNAME.run("whoa"), Ok(()));
    assert!(USERNAME.run("whoa whoa").is_err());
    assert!(USERNAME.run("").is_err());

    assert_eq!(NICKNAME.run("whoa"), Ok(()));
    assert!(NICKNAME.run("whoa whoa").is_ok());
    assert!(NICKNAME.run("").is_err());

    assert!(EMAIL.run("").is_err());
    assert!(EMAIL.run("example@example.com").is_ok());
}
