use log::{
    info,
    error
};
use tokio::{
    task,
    sync::mpsc
};

use crate::data_model::{
    SharedClient, 
    Similarity
};
use crate::helpers::{
    self,
    Void,
    GenericError
};

pub fn start(mongo_client: &SharedClient, mut signature_ready_rx: mpsc::UnboundedReceiver<String>, similarities_ready_tx: &mpsc::UnboundedSender<String>) {
    let mongo_client_clone = mongo_client.clone();
    let similarities_ready_tx_clone = similarities_ready_tx.clone();

    let _ = task::spawn(async move {
        while let Some(handle) = signature_ready_rx.recv().await {
            let mongo_client_clone2 = mongo_client_clone.clone();
            let similarities_ready_tx_clone2 = similarities_ready_tx_clone.clone();

            let _ = task::spawn(async move {
                if let Err(e) = update_similarities_for(&handle, &mongo_client_clone2).await {
                    error!("[{}] Failed to get or store similarities: {}", handle, e);
                } else if let Err(e) = similarities_ready_tx_clone2.send(handle.to_owned()) {
                    error!("[{}] Failed to send on `similarities_ready_tx`: {}", handle, e);
                }
            });
        }
    });
}

async fn update_similarities_for(handle: &str, mongo_client: &SharedClient) -> Void {
    info!("[{}] Computing similarities.", handle);

    let signatures = mongo_client.get_all_signatures().await?;
    let mut similarities = Vec::<Similarity>::with_capacity(signatures.len());

    if let Some(requested_signature) = signatures.iter().find(|s| s.user_handle == handle.to_lowercase()) {
        for signature in &signatures {
            if requested_signature.user_handle == signature.user_handle {
                continue;
            }

            let (source_handle, target_handle) = helpers::compute_similarity_handles(&requested_signature.user_handle, &signature.user_handle);
            let strength = helpers::compute_similarity_strength(&requested_signature.signature, &signature.signature);

            similarities.push(Similarity { source_handle, target_handle, strength });
        }
    } else {
        return Err(Box::new(GenericError::from(format!("Could not find signature for handle {}.", handle))));
    }

    mongo_client.insert_similarities(&similarities).await?;

    Ok(())
}