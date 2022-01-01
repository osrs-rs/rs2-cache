mod common;

mod osrs {
    const REGION_GRID_LUMBRIDGE: u16 = 12850;

    use super::common;

    use osrscache::loader::osrs::LocationLoader;
    #[test]
    fn load_locations() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;

        // XTEA keys
        let keys: [u32; 4] = [1766500218, 1050654932, 397022681, 1618041309];

        let mut location_loader = LocationLoader::new(&cache);
        let location_def = location_loader.load(REGION_GRID_LUMBRIDGE, &keys)?;

        assert_eq!(location_def.region_x, 50);
        assert_eq!(location_def.region_y, 50);
        assert_eq!(location_def.region_base_coords(), (3200, 3200));
        assert_eq!(location_def.data.len(), 4730);

        Ok(())
    }
}
