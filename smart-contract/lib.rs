use concordium_sdk::{
    contract::{
        Contract,
        ContractError,
        Context,
        EntryPoint,
        Receipt,
    },
    crypto::{
        Hash,
        PublicKey,
        Signature,
    },
    storage::{
        DataType,
        Storage,
    },
};

#[derive(Debug, PartialEq, Eq)]
enum State {
    Active,
    Funded,
    Cancelled,
}

#[derive(Debug, PartialEq, Eq)]
struct Campaign {
    creator: PublicKey,
    goal: u64,
    current_total: u64,
    state: State,
}

struct CrowdFunding {
    campaigns: Storage<PublicKey, Campaign>,
}

impl Contract for CrowdFunding {
    fn create(ctx: &Context) -> Result<(), ContractError> {
        let storage = CrowdFunding {
            campaigns: Storage::new(ctx.storage_ctx()),
        };
        ctx.set_state(Box::new(storage))
    }

    fn call(ctx: &Context) -> Result<Receipt, ContractError> {
        let storage: &mut CrowdFunding = ctx.state_mut();

        let entry_point: EntryPoint = ctx.entry_point()?;
        match entry_point {
            EntryPoint::CreateCampaign(creator, goal) => {
                let campaign = Campaign {
                    creator,
                    goal,
                    current_total: 0,
                    state: State::Active,
                };
                storage.campaigns.insert(ctx.caller(), campaign);
                Ok(Receipt::Success)
            }
            EntryPoint::Contribute(campaign_key, contribution) => {
                let campaign = storage.campaigns.get(campaign_key)?;
                if campaign.state != State::Active {
                    return Err(ContractError::InvalidState);
                }
                let mut updated_campaign = campaign;
                updated_campaign.current_total += contribution;
                if updated_campaign.current_total >= updated_campaign.goal {
                    updated_campaign.state = State::Funded;
                }
                storage.campaigns.insert(campaign_key, updated_campaign);
                Ok(Receipt::Success)
            }
            EntryPoint::CancelCampaign(campaign_key) => {
                let campaign = storage.campaigns.get(campaign_key)?;
                if campaign.state != State::Active {
                    return Err(ContractError::InvalidState);
                }
                if campaign.creator != ctx.caller() {
                    return Err(ContractError::Unauthorized);
                }
                let mut updated_campaign = campaign;
                updated_campaign.state = State::Cancelled;
                storage.campaigns.insert(campaign_key, updated_campaign);
                Ok(Receipt::Success)
            }
            EntryPoint::Refund(campaign_key, contributor) => {
                let campaign = storage.campaigns.get(campaign_key)?;
                if campaign.state != State::Cancelled {
                    return Err(ContractError::InvalidState);
                }
                let contribution = campaign.contributions.remove(contributor)?;
                let mut contributor_account = ctx.account(contributor)?;
                contributor_account.balance += contribution;
                ctx.set_account(contributor, contributor_account)?;
                Ok(Receipt::Success)
            }
            EntryPoint::CloseCampaign(campaign_key) => {
                let campaign = storage.campaigns.get(campaign_key)?;
                if campaign.state != State::Funded {
                    return Err(ContractError::InvalidState);
                }
                let mut updated_campaign = campaign;
                updated_campaign.state = State::Closed;
                storage.campaigns.insert(campaign_key, updated_campaign);
                Ok(Receipt::Success)
            }
        }
    }
}

fn main() {
    Contract::deploy(CrowdFunding {
        campaigns: Storage::new(DataType::Private),
    });
}

