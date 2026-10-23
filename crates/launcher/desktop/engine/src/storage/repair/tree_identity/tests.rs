use super::*;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
struct TestPlan;
impl TreePlan for TestPlan {
    fn marker_name(&self) -> String {
        ".test-tree.json".into()
    }
}

#[test]
fn direct_tree_marker_refuses_a_copied_owner_and_is_exclusive() {
    let temp = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    for root in [temp.path(), other.path()] {
        std::fs::write(root.join(".cimmeria-install.json"), b"same marker bytes").unwrap();
    }
    let owner = File::open(temp.path().join(".cimmeria-install.json")).unwrap();
    assert!(write_owned(other.path(), &TestPlan, Role::Replacement, &owner).is_err());
    assert!(!other.path().join(name(&TestPlan)).exists());
    write_owned(temp.path(), &TestPlan, Role::Replacement, &owner).unwrap();
    let before = std::fs::read(temp.path().join(name(&TestPlan))).unwrap();
    assert!(write_owned(temp.path(), &TestPlan, Role::Original, &owner).is_err());
    assert_eq!(
        std::fs::read(temp.path().join(name(&TestPlan))).unwrap(),
        before
    );
}
