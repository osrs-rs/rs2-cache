mod common;

mod osrs {
    use super::common;

    #[test]
    fn read_from_ref_table() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;

        let archive = cache.read(255, 10)?;

        let hash = common::hash(&archive);
        assert_eq!(&hash, "64fb9fcf381a547bb7beafbc3b7ba4fd847f21ef");
        assert_eq!(archive.len(), 77);

        Ok(())
    }

    #[test]
    fn read_from_0_16() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;

        let archive = cache.read(0, 16)?;

        let hash = common::hash(&archive);
        assert_eq!(&hash, "ad53ed574c3539400c822a9fc4c028b3e5e50e33");
        assert_eq!(archive.len(), 1543);

        Ok(())
    }

    #[test]
    fn read_from_0_191() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;

        let archive = cache.read(0, 191)?;

        let hash = common::hash(&archive);
        assert_eq!(&hash, "cd459f6ccfbd81c1e3bfadf899624f2519e207a9");
        assert_eq!(archive.len(), 2055);

        Ok(())
    }

    #[test]
    fn read_from_2_10() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;

        let archive = cache.read(2, 10)?;

        let hash = common::hash(&archive);
        assert_eq!(&hash, "c6ee1518e9a39a42ecaf946c6c84a942cb3102f4");
        assert_eq!(archive.len(), 260_537);

        Ok(())
    }

    #[test]
    fn read_from_7_24918() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;

        let archive = cache.read(7, 24918)?;

        let hash = common::hash(&archive);
        assert_eq!(&hash, "fe91e9e9170a5a05ed2684c1db1169aa7ef4906e");
        assert_eq!(archive.len(), 803);

        Ok(())
    }

    #[test]
    fn read_from_3_278() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;

        let archive = cache.read(3, 278)?;

        assert_eq!(archive.len(), 512);

        Ok(())
    }

    #[test]
    fn read_from_0_1077() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;

        let archive = cache.read(0, 1077)?;

        assert_eq!(archive.len(), 1024);

        Ok(())
    }

    #[test]
    fn read_from_2_25000_fails() -> osrscache::Result<()> {
        let cache = common::osrs::setup()?;

        assert!(cache.read(2, 25_000).is_err());

        Ok(())
    }
}
