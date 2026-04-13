use super::*;

#[test]
fn bob_cannot_write_in_finance() {
    let (mut fs, _, _, mut bob, ..) = setup();
    let result = try_run("touch finance/hack.md", &mut fs, &mut bob);
    assert!(result.is_err(), "bob should not write in finance/");
}

#[test]
fn alice_cannot_overwrite_bob_private_file() {
    let (mut fs, _, mut alice, ..) = setup();
    let result = try_run(
        "write home/bob/notes.md hacked!",
        &mut fs,
        &mut alice,
    );
    assert!(
        result.is_err(),
        "alice should not overwrite bob's private file"
    );
}

#[test]
fn carol_cannot_delete_engineering_files() {
    let (mut fs, _, _, _, mut carol, _) = setup();
    let result = try_run("rm engineering/design.md", &mut fs, &mut carol);
    assert!(result.is_err(), "carol cannot delete in engineering/");
}

#[test]
fn bob_cannot_write_in_alice_home() {
    let (mut fs, _, _, mut bob, ..) = setup();
    let result = try_run("touch home/alice/hacked.md", &mut fs, &mut bob);
    assert!(result.is_err(), "bob cannot create files in alice's home");
}

#[test]
fn carol_cannot_mkdir_in_engineering() {
    let (mut fs, _, _, _, mut carol, _) = setup();
    let result = try_run("mkdir engineering/subdir", &mut fs, &mut carol);
    assert!(result.is_err(), "carol cannot mkdir in engineering/");
}
