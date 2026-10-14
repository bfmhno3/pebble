use std::collections::{BTreeSet, HashSet, VecDeque};

fn main() {
    let items = [5, 1, 5, 3, 1];

    let mut hash: HashSet<i32> = HashSet::new();
    let mut tree: BTreeSet<i32> = BTreeSet::new();
    for item in items {
        println!("insert {item}: hash new? {}  tree new? {}", hash.insert(item), tree.insert(item));
    }
    println!("tree (sorted): {:?}", tree);
    let mut from_hash: Vec<_> = hash.into_iter().collect();
    from_hash.sort();
    println!("hash (sorted by us): {from_hash:?}");

    let mut queue: VecDeque<i32> = VecDeque::new();
    queue.push_back(1);
    queue.push_back(2);
    queue.push_front(0);
    let front = queue.pop_front();
    println!("pop_front = {front:?}, rest = {queue:?}");
}
