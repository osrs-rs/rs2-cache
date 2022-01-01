mod common;

mod osrs {
    use super::common;

    #[test]
    fn load_woodsman_tutor() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;
        let npc_loader = common::osrs::load_npcs(&cache)?;

        let npc = npc_loader.load(3226).unwrap();

        assert_eq!(npc.name, "Woodsman tutor");
        assert!(npc.interactable);

        Ok(())
    }

    #[test]
    fn load_tool_leprechaun() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;
        let npc_loader = common::osrs::load_npcs(&cache)?;

        let npc = npc_loader.load(0).unwrap();

        assert_eq!(npc.name, "Tool Leprechaun");
        assert!(npc.interactable);

        Ok(())
    }

    #[test]
    fn incorrect_npc_id() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;
        let npc_loader = common::osrs::load_npcs(&cache)?;

        let npc = npc_loader.load(65_535);

        assert!(npc.is_none());

        Ok(())
    }
}
