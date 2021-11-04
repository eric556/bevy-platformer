use bevy::{
    prelude::*,
    asset::{AssetLoader, LoadedAsset}
};
pub mod ldtk_json;

#[derive(Default)]
pub struct LdtkAssetLoader;

impl AssetLoader for LdtkAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let custom_asset = serde_json::from_slice::<ldtk_json::Project>(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(custom_asset));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ldtk"]
    }
}

pub struct LdtkLoaderPlugin;

impl Plugin for LdtkLoaderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<ldtk_json::Project>();
        app.init_asset_loader::<LdtkAssetLoader>();
    }
}