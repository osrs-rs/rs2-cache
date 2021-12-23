#![allow(dead_code)]

use sha1::Sha1;

pub fn hash(buffer: &[u8]) -> String {
    let mut m = Sha1::new();

    m.update(&buffer);
    m.digest().to_string()
}

pub mod osrs {
    use osrscache::Cache;

    use osrscache::loader::osrs::{
        InventoryLoader, ItemLoader, NpcLoader, ObjectLoader, VarbitLoader,
    };
    pub fn setup() -> osrscache::Result<Cache> {
        Cache::new("./data/osrs_cache")
    }

    pub fn load_items(cache: &Cache) -> osrscache::Result<ItemLoader> {
        ItemLoader::new(cache)
    }

    pub fn load_npcs(cache: &Cache) -> osrscache::Result<NpcLoader> {
        NpcLoader::new(cache)
    }
    pub fn load_objects(cache: &Cache) -> osrscache::Result<ObjectLoader> {
        ObjectLoader::new(cache)
    }

    pub fn load_inventories(cache: &Cache) -> osrscache::Result<InventoryLoader> {
        InventoryLoader::new(cache)
    }

    pub fn load_varbits(cache: &Cache) -> osrscache::Result<VarbitLoader> {
        VarbitLoader::new(cache)
    }
}
