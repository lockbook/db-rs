use db_rs::{
    View,
    config::Config,
    errors::Result,
    views::{
        composite_view::{Composite, Schema},
        hashmap::DbHashMap,
        hashmap_map::DbHashMapMap,
        hashmap_set::DbHashMapSet,
        option::DbOption,
        vec::DbVec,
    },
};

#[test]
fn collection_views_track_commits_replay_and_empty_snapshots() {
    check::<DbHashMap<u64, u64>>(|v| v.insert(1, 2).map(|_| ()), |v| v.clear());
    check::<DbHashMapMap<u64, u64, u64>>(|v| v.insert(1, 2, 3).map(|_| ()), |v| v.clear());
    check::<DbHashMapSet<u64, u64>>(|v| v.insert(1, 2).map(|_| ()), |v| v.clear());
    check::<DbVec<u64>>(|v| v.push(1), |v| v.clear());
    check::<DbOption<u64>>(|v| v.replace(1).map(|_| ()), |v| v.take().map(|_| ()));
}

#[test]
fn composite_tracks_only_changed_children_and_preserves_their_sequences() {
    let config = Config::test();
    let mut db = Composite::<Outer>::init(&config).unwrap();
    let mut reader = Composite::<Outer>::init(&config).unwrap();
    assert_eq!(revisions(&db), (0, 0, 0, 0, 0));

    let tx = db.write_tx().unwrap();
    db.schema.inner.schema.metadata.insert(1, 2).unwrap();
    assert_eq!(revisions(&db), (0, 0, 0, 0, 0));
    tx.end_tx(&mut db).unwrap();
    assert_eq!(revisions(&db), (1, 1, 1, 0, 0));

    let tx = db.write_tx().unwrap();
    db.schema.unrelated.replace(42).unwrap();
    tx.end_tx(&mut db).unwrap();
    assert_eq!(revisions(&db), (2, 1, 1, 0, 2));
    db.snapshot().unwrap();
    let reopened = Composite::<Outer>::init(&config).unwrap();
    assert_eq!(revisions(&reopened), (2, 1, 1, 0, 2));

    let tx = db.write_tx().unwrap();
    db.schema.inner.schema.metadata.clear().unwrap();
    tx.end_tx(&mut db).unwrap();
    db.snapshot().unwrap();
    assert_eq!(revisions(&db), (3, 3, 3, 0, 2));

    reader.write_tx().unwrap().end_tx(&mut reader).unwrap();
    assert_eq!(revisions(&reader), revisions(&db));
    let reopened = Composite::<Outer>::init(&config).unwrap();
    assert_eq!(revisions(&reopened), revisions(&db));
    assert!(reopened.schema.inner.schema.metadata.is_empty());
}

fn check<V: View + Default>(
    edit: impl Fn(&mut V) -> Result<()>,
    clear: impl Fn(&mut V) -> Result<()>,
) {
    let config = Config::test();
    let mut db = V::init(&config).unwrap();
    let mut reader = V::init(&config).unwrap();
    assert_eq!(db.last_modified(), 0);

    let tx = db.write_tx().unwrap();
    edit(&mut db).unwrap();
    assert_eq!(db.last_modified(), 0);
    tx.end_tx(&mut db).unwrap();
    assert_eq!(db.last_modified(), 1);

    db.write_tx().unwrap().end_tx(&mut db).unwrap();
    assert_eq!(db.last_modified(), 1);
    reader.write_tx().unwrap().end_tx(&mut reader).unwrap();
    assert_eq!(reader.last_modified(), 1);

    let tx = db.write_tx().unwrap();
    clear(&mut db).unwrap();
    tx.end_tx(&mut db).unwrap();
    assert_eq!(db.last_modified(), 2);
    db.snapshot().unwrap();
    assert_eq!(db.last_modified(), 2);
    let reopened = V::init(&config).unwrap();
    assert_eq!(reopened.last_modified(), 2);
    reader.write_tx().unwrap().end_tx(&mut reader).unwrap();
    assert_eq!(reader.last_modified(), 2);
}

#[derive(Default)]
struct Tables {
    metadata: DbHashMap<u64, u64>,
    untouched: DbOption<u64>,
}

impl Schema for Tables {
    fn views_mut(&mut self) -> impl AsMut<[&mut dyn View]> {
        let views: [&mut dyn View; 2] = [&mut self.metadata, &mut self.untouched];
        views
    }
}

#[derive(Default)]
struct Outer {
    inner: Composite<Tables>,
    unrelated: DbOption<u64>,
}

impl Schema for Outer {
    fn views_mut(&mut self) -> impl AsMut<[&mut dyn View]> {
        let views: [&mut dyn View; 2] = [&mut self.inner, &mut self.unrelated];
        views
    }
}

fn revisions(db: &Composite<Outer>) -> (u64, u64, u64, u64, u64) {
    (
        db.last_modified(),
        db.schema.inner.last_modified(),
        db.schema.inner.schema.metadata.last_modified(),
        db.schema.inner.schema.untouched.last_modified(),
        db.schema.unrelated.last_modified(),
    )
}
