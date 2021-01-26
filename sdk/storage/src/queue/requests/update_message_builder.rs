use crate::queue::*;
use crate::responses::*;
use azure_core::headers::add_optional_header;
use azure_core::prelude::*;
use std::convert::TryInto;

#[derive(Debug, Clone)]
pub struct UpdateMessageBuilder<'a> {
    queue_client: &'a QueueClient,
    visibility_timeout: VisibilityTimeout,
    timeout: Option<Timeout>,
    client_request_id: Option<ClientRequestId<'a>>,
}

impl<'a> UpdateMessageBuilder<'a> {
    pub(crate) fn new(
        queue_client: &'a QueueClient,
        visibility_timeout: impl Into<VisibilityTimeout>,
    ) -> Self {
        UpdateMessageBuilder {
            queue_client,
            visibility_timeout: visibility_timeout.into(),
            timeout: None,
            client_request_id: None,
        }
    }

    setters! {
        timeout: Timeout => Some(timeout),
        client_request_id: ClientRequestId<'a> => Some(client_request_id),
    }

    pub async fn execute(
        &self,
        pop_receipt: &dyn PopReceipt,
        new_body: impl AsRef<str>,
    ) -> Result<UpdateMessageResponse, Box<dyn std::error::Error + Sync + Send>> {
        let mut url = self
            .queue_client
            .queue_url()?
            .join("messages/")?
            .join(pop_receipt.message_id())?;

        url.query_pairs_mut()
            .append_pair("popreceipt", pop_receipt.pop_receipt());
        self.visibility_timeout.append_to_url_query(&mut url);
        self.timeout.append_to_url_query(&mut url);

        trace!("url == {}", url.as_str());

        // since the format is fixed we just decorate the message with the tags.
        // This could be made optional in the future and/or more
        // stringent.
        let message = format!(
            "<QueueMessage><MessageText>{}</MessageText></QueueMessage>",
            new_body.as_ref()
        );

        debug!("message about to be put == {}", message);

        let request = self.queue_client.storage_client().prepare_request(
            url.as_str(),
            &http::method::Method::PUT,
            &|mut request| {
                request = add_optional_header(&self.client_request_id, request);
                request
            },
            Some(message.into()),
        )?;

        let response = self
            .queue_client
            .storage_client()
            .storage_account_client()
            .http_client()
            .execute_request_check_status(request.0, http::status::StatusCode::NO_CONTENT)
            .await?;

        Ok((&response).try_into()?)
    }
}
