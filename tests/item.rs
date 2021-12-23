mod common;

mod osrs {
    use super::common;

    #[test]
    fn load_item_blue_partyhat() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;
        let item_loader = common::osrs::load_items(&cache)?;

        let item = item_loader.load(1042).unwrap();

        assert_eq!(item.name, "Blue partyhat");
        assert!(!item.stackable);
        assert!(!item.members_only);

        Ok(())
    }

    #[test]
    fn load_item_magic_logs() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;
        let item_loader = common::osrs::load_items(&cache)?;

        let item = item_loader.load(1513).unwrap();

        assert_eq!(item.name, "Magic logs");
        assert!(!item.stackable);
        assert!(item.members_only);

        Ok(())
    }

    #[test]
    fn load_item_logs_noted() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;
        let item_loader = common::osrs::load_items(&cache)?;

        let item = item_loader.load(1512).unwrap();

        assert!(item.stackable);
        assert!(!item.members_only);

        Ok(())
    }

    #[test]
    fn incorrect_item_id() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;
        let item_loader = common::osrs::load_items(&cache)?;

        let item = item_loader.load(65_535);

        assert!(item.is_none());

        Ok(())
    }
}
