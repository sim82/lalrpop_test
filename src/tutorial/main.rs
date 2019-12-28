#[macro_use]
extern crate lalrpop_util;

fn main() {
    println!("Hello, world!");
}

lalrpop_mod!(pub calculator1); // synthesized by LALRPOP

#[test]
fn calculator1() {
    assert!(calculator1::TermParser::new().parse("22").is_ok());
    assert!(calculator1::TermParser::new().parse("(22)").is_ok());
    assert!(calculator1::TermParser::new().parse("((((22))))").is_ok());
    assert!(calculator1::TermParser::new().parse("((22)").is_err());
}

lalrpop_mod!(pub calculator2); // synthesized by LALRPOP

#[test]
fn calculator2() {
    // calculator2::TermParser::new().parse(")22").unwrap();
    assert!(calculator2::TermParser::new().parse("(22)").is_ok());
    assert!(calculator2::TermParser::new().parse("(22)").is_ok());
    assert!(calculator2::TermParser::new().parse("((((22))))").is_ok());
    assert!(calculator2::TermParser::new().parse("((22)").is_err());
}

lalrpop_mod!(pub calculator4);
pub mod ast;

#[test]
fn calculator4() {
    let expr = calculator4::ExprParser::new()
        .parse("22 * 44 + 66")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "((22 * 44) + 66)");
}

lalrpop_mod!(pub calculator5);

#[test]
fn calculator5() {
    let expr = calculator5::ExprsParser::new().parse("").unwrap();
    assert_eq!(&format!("{:?}", expr), "[]");

    let expr = calculator5::ExprsParser::new()
        .parse("22 * 44 + 66")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * 44) + 66)]");

    let expr = calculator5::ExprsParser::new()
        .parse("22 * 44 + 66,")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * 44) + 66)]");

    let expr = calculator5::ExprsParser::new()
        .parse("22 * 44 + 66, 13*3")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * 44) + 66), (13 * 3)]");

    let expr = calculator5::ExprsParser::new()
        .parse("22 * 44 + 66, 13*3,")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * 44) + 66), (13 * 3)]");
}

lalrpop_mod!(pub calculator6);

#[test]
fn calculator6() {
    let mut errors = Vec::new();

    let expr = calculator6::ExprsParser::new()
        .parse(&mut errors, "22 * + 3")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * error) + 3)]");

    let expr = calculator6::ExprsParser::new()
        .parse(&mut errors, "22 * 44 + 66, *3")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * 44) + 66), (error * 3)]");

    let expr = calculator6::ExprsParser::new()
        .parse(&mut errors, "*")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[(error * error)]");

    assert_eq!(errors.len(), 4);
}

lalrpop_mod!(pub lang1);

#[test]
fn lang1() {
    let mut errors = Vec::new();

    let expr = lang1::ExprsParser::new()
        .parse(&mut errors, "22 * + 3")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * error) + 3)]");

    let expr = lang1::ExprsParser::new()
        .parse(&mut errors, "22 * 44 + 66, *3")
        .unwrap();
    assert_eq!(&format!("{:?}", expr), "[((22 * 44) + 66), (error * 3)]");

    let expr = lang1::ExprsParser::new().parse(&mut errors, "*").unwrap();
    assert_eq!(&format!("{:?}", expr), "[(error * error)]");

    assert_eq!(errors.len(), 4);
}
