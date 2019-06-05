//! Metalog runtime, see https://github.com/PACTCare/Stars-Network/blob/master/WHITEPAPER.md#--starlog--substrate-
use support::{decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageValue, StorageMap, traits::Currency};
use parity_codec::{Encode, Decode};
use system::ensure_signed;
use rstd::vec::Vec;
use runtime_primitives::traits::{As};

//FIXME: needs to be removed for building the runtime
//use runtime_io::{with_storage, StorageOverlay, ChildrenStorageOverlay};

const ERR_DID_ALREADY_CLAIMED: &str = "This DID has already been claimed.";
const ERR_DID_NOT_EXIST: &str = "This DID does not exist";
const ERR_DID_NO_OWNER: &str = "No one owens this did";

const ERR_UN_ALREADY_CLAIMED: &str = "This unique name has already been claimed.";

const ERR_LICENSE_INVALID: &str = "Invalid license code";

const ERR_OVERFLOW: &str = "Overflow adding new metadata";
const ERR_UNDERFLOW: &str = "Underflow removing metadata";

const ERR_NOT_OWNER: &str = "You are not the owner";

const ERR_OPEN_NAME_ACCOUNT_CLAIMED: &str = "Unique name account already claimed";

const ERR_BYTEARRAY_LIMIT_DID: &str = "DID bytearray is too large";
const ERR_BYTEARRAY_LIMIT_LOCATION: &str = "Location bytearray is too large";
const ERR_BYTEARRAY_LIMIT_NAME: &str = "Name bytearray is too large";

const BYTEARRAY_LIMIT_DID: usize = 100;
const BYTEARRAY_LIMIT_LOCATION: usize = 100;
const BYTEARRAY_LIMIT_NAME: usize = 50;

const DELETE_LICENSE: u16 = 1;

/// The module's configuration traits are timestamp and balance
pub trait Trait: timestamp::Trait + balances::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// Key metalog struct
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Metalog<Time> {
	pub did: Vec<u8>, //= primary key, can't be changed
	pub unique_name: Vec<u8>, // Default = 0
	pub license_code: u16, // 0 = no license code, 1 = delete request     
	pub storage_location: Vec<u8>,  
	pub time: Time,
}

decl_storage! {
	trait Store for Module<T: Trait> as Metalog {
		/// Array of personal owned metalog data
		OwnedMetaArray get(metadata_of_owner_by_index): map (T::AccountId, u64) => Metalog<T::Moment>;

		/// Number of stored metalogs per account 
		OwnedMetaCount get(owner_meta_count): map T::AccountId => u64;

		/// Index of DID
		OwnedMetaIndex: map Vec<u8> => u64;

		/// Query for unique names
		UnMeta get(meta_of_un): map Vec<u8> => Metalog<T::Moment>;
		UnOwner get(owner_of_un): map Vec<u8> => Option<T::AccountId>;

		/// Query by DIDs 
		DidMeta get(meta_of_did): map Vec<u8> => Metalog<T::Moment>;
		DidOwner get(owner_of_did): map Vec<u8> => Option<T::AccountId>;

		/// Account which gets all the money for the unique name
		UniqueNameAccount get(unique_name_account): T::AccountId;
	}

	//FIXME: needs to be removed for building the runtime
	// add_extra_genesis {
    //     config(metalog): Vec<(T::AccountId, u16)>;
    //     build(|storage: &mut StorageOverlay, _: &mut ChildrenStorageOverlay, config: &GenesisConfig<T>| {
    //         with_storage(storage, || {
    //             for &(ref acct, license_code) in &config.metalog {
	// 				let did = vec![1,2,3];
	// 				let time = <timestamp::Module<T>>::now();	
	// 				let mut default_name = Vec::new();
	// 				default_name.push(0);
	// 				let new_metadata = Metalog {
	// 					did: did.clone(),
	// 					unique_name: default_name, 
	// 					license_code,
	// 					storage_location: did.clone(),
	// 					time,
	// 				};
    //                 let _ = <Module<T>>::_owner_store(acct.clone(), new_metadata);
    //             }
    //         });
    //     });
    // }
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// For deposit events
		fn deposit_event<T>() = default;

		/// Initialize unique name account
		pub fn init_unique_name_account(origin) -> Result {
			let sender = ensure_signed(origin)?;
			ensure!(!<UniqueNameAccount<T>>::exists(), ERR_OPEN_NAME_ACCOUNT_CLAIMED);
			<UniqueNameAccount<T>>::put(sender);
			Ok(())
		}

		/// Store initial metalog
        fn create_metalog(
			origin, 
			did: Vec<u8>, 
			license_code: u16, 
			storage_location: Vec<u8>) -> Result {

            let sender = ensure_signed(origin)?;

			ensure!(did.len() <= BYTEARRAY_LIMIT_DID, ERR_BYTEARRAY_LIMIT_DID);
			ensure!(storage_location.len() <= BYTEARRAY_LIMIT_LOCATION, ERR_BYTEARRAY_LIMIT_LOCATION);
			ensure!(!<DidOwner<T>>::exists(&did), ERR_DID_ALREADY_CLAIMED);
			ensure!(license_code != DELETE_LICENSE, ERR_LICENSE_INVALID);

			let time = <timestamp::Module<T>>::now();

			let mut default_name = Vec::new();
			default_name.push(0);
			let new_metadata = Metalog {
				did,
				unique_name: default_name, 
				license_code,
				storage_location,
				time,
			};

			Self::_owner_store(sender.clone(), new_metadata.clone())?;
			Self::deposit_event(RawEvent::Stored(sender, new_metadata.time, new_metadata.did));
            Ok(())
        }

		/// Transfer the ownership, Payment will be implemented in smart contracts
		fn transfer_ownership(origin, receiver: T::AccountId, did: Vec<u8>) -> Result {
			let sender = ensure_signed(origin)?;
			Self::_check_did_ownership(sender.clone(), &did)?;
			Self::_transfer(sender.clone(), receiver.clone(), &did)?;

			Self::deposit_event(RawEvent::TransferOwnership(sender, receiver, did));
			Ok(())
		}

		/// Buy a unique name
		pub fn buy_unique_name(origin, did: Vec<u8>, unique_name: Vec<u8>)-> Result{
			let sender = ensure_signed(origin)?;

			Self::_check_did_ownership(sender.clone(), &did)?;

			ensure!(unique_name.len() <= BYTEARRAY_LIMIT_NAME, ERR_BYTEARRAY_LIMIT_NAME);

			ensure!(!<UnOwner<T>>::exists(&unique_name), ERR_UN_ALREADY_CLAIMED);
			Self::_pay_name(sender.clone())?;

			let mut metalog = Self::meta_of_did(&did);
			metalog.unique_name = unique_name.clone();

			let meta_index = <OwnedMetaIndex<T>>::get(&did);
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index -1), &metalog);
			<DidMeta<T>>::insert(&did, &metalog);

			<UnMeta<T>>::insert(&metalog.unique_name, &metalog);
			<UnOwner<T>>::insert(&metalog.unique_name, &sender);
			
			Self::deposit_event(RawEvent::NameUpdated(sender, did, unique_name));
			Ok(())
		}

		/// Change license code
		pub fn change_license_code(origin, did: Vec<u8>, license_code: u16)-> Result{
			let sender = ensure_signed(origin)?;

			Self::_check_did_ownership(sender.clone(), &did)?;
			let mut metadata = Self::meta_of_did(&did);
			metadata.license_code = license_code.clone();

			let meta_index = <OwnedMetaIndex<T>>::get(&did);
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index -1), &metadata);
			<DidMeta<T>>::insert(&did, &metadata);

			Self::deposit_event(RawEvent::LicenseUpdated(sender, did, license_code));
			Ok(())
		}

		/// Change storage location
		pub fn change_storage_location(origin, did: Vec<u8>, storage_location: Vec<u8>)-> Result{
			let sender = ensure_signed(origin)?;

			ensure!(storage_location.len() <= BYTEARRAY_LIMIT_LOCATION, ERR_BYTEARRAY_LIMIT_LOCATION);

			Self::_check_did_ownership(sender.clone(), &did)?;
			let mut metadata = Self::meta_of_did(&did);
			metadata.storage_location = storage_location.clone();

			let meta_index = <OwnedMetaIndex<T>>::get(&did);
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index -1), &metadata);
			<DidMeta<T>>::insert(&did, &metadata);

			Self::deposit_event(RawEvent::LocationUpdated(sender, did, storage_location));
			Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as timestamp::Trait>::Moment, 
    {
        Stored(AccountId, Moment, Vec<u8>),
		TransferOwnership(AccountId, AccountId, Vec<u8>),
		LicenseUpdated(AccountId, Vec<u8>,u16),
		LocationUpdated(AccountId, Vec<u8>,Vec<u8>),
		NameUpdated(AccountId, Vec<u8>,Vec<u8>),
	}
);

impl<T: Trait> Module<T> { 
	/// store metalog
	fn _owner_store(sender: T::AccountId, metalog: Metalog<T::Moment>) -> Result {
		let count = Self::owner_meta_count(&sender);
		let updated_count = count.checked_add(1).ok_or(ERR_OVERFLOW)?;

		<OwnedMetaArray<T>>::insert((sender.clone(), count), &metalog);
		<OwnedMetaCount<T>>::insert(&sender, updated_count);
		<OwnedMetaIndex<T>>::insert(&metalog.did, updated_count);

		<DidMeta<T>>::insert(&metalog.did, &metalog);
		<DidOwner<T>>::insert(&metalog.did, &sender);

		Ok(())
	}

	/// Checks the ownership rights
	fn _check_did_ownership(sender: T::AccountId, did: &Vec<u8>) -> Result {
		ensure!(<DidMeta<T>>::exists(did), ERR_DID_NOT_EXIST);
		let owner = Self::owner_of_did(did).ok_or(ERR_DID_NO_OWNER)?;
		ensure!(owner == sender, ERR_NOT_OWNER);

		Ok(())
	}

	/// Transfer ownership
	fn _transfer(sender: T::AccountId, receiver: T::AccountId, did: &Vec<u8>) -> Result {
		let receiver_total_count = Self::owner_meta_count(&receiver);
		let new_receiver_count = receiver_total_count.checked_add(1).ok_or(ERR_OVERFLOW)?;

		let sender_total_count = Self::owner_meta_count(&sender);
		let new_sender_count = sender_total_count.checked_sub(1).ok_or(ERR_UNDERFLOW)?;

		let meta_index = <OwnedMetaIndex<T>>::get(did);
		let meta_object = <OwnedMetaArray<T>>::get((sender.clone(), new_sender_count));

		if meta_index != new_sender_count {
			<OwnedMetaArray<T>>::insert((sender.clone(), meta_index), &meta_object);
			<OwnedMetaIndex<T>>::insert(&meta_object.did, meta_index);
		}

		// if un is not the default un
		let mut default_name = Vec::new();
		default_name.push(0);
		if meta_object.unique_name != default_name {
			<UnOwner<T>>::insert(did, &receiver);
		}

		<DidOwner<T>>::insert(did, &receiver);

		<OwnedMetaIndex<T>>::insert(did, receiver_total_count);

		<OwnedMetaArray<T>>::remove((sender.clone(), new_sender_count));
		<OwnedMetaArray<T>>::insert((receiver.clone(), receiver_total_count), meta_object);

		<OwnedMetaCount<T>>::insert(&sender, new_sender_count);
		<OwnedMetaCount<T>>::insert(&receiver, new_receiver_count);

		Ok(())
	}

	/// Payment for unique name
	fn _pay_name(sender: T::AccountId)-> Result{
		let price = <T::Balance as As<u64>>::sa(1000);
		let name_account = Self::unique_name_account();

		// transfer() function verifies and writes
		<balances::Module<T> as Currency<_>>::transfer(&sender, &name_account, price)?;

		Ok(())
	}
}

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use primitives::{Blake2Hasher, H256};
	use runtime_io::{with_externalities};
	use runtime_primitives::{
		testing::{Digest, DigestItem, Header},
		traits::{BlakeTwo256, IdentityLookup},
		BuildStorage,
	};
	use support::{assert_noop, assert_ok, impl_outer_origin};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;

	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}
	impl balances::Trait for Test {
		type Balance = u64;
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type Event = ();
		type TransactionPayment = ();
		type DustRemoval = ();
		type TransferPayment = ();
	}

	impl timestamp::Trait for Test {
		type Moment = u64;
		type OnTimestampSet = ();
	}

	impl Trait for Test {
		type Event = ();
	}

	type Balances = balances::Module<Test>;
	type Metalog = Module<Test>;

	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
        t.extend(balances::GenesisConfig::<Test>::default().build_storage().unwrap().0);
        t.extend(GenesisConfig::<Test> {
            metalog: vec![(0, 0)],
        }.build_storage().unwrap().0);
        t.into()
	}

	#[test]
	fn init_unique_name_account_works() {
		with_externalities(&mut new_test_ext(), || {
			assert_ok!(Metalog::init_unique_name_account(Origin::signed(1)));
		});
	}

	#[test]
	fn create_metalog_works() {
		with_externalities(&mut new_test_ext(), || {
			let did_new = vec![1,2];
			let did_claimed = vec![1,2,3];
			let mut did_too_long = did_new.clone();
			let mut location_too_long = did_new.clone();
			for _i in 1..100 {
				did_too_long.push(2);
				location_too_long.push(2);
			}
			assert_noop!(Metalog::create_metalog(Origin::signed(20), did_new.clone() , DELETE_LICENSE, did_new.clone()), ERR_LICENSE_INVALID);
			assert_noop!(Metalog::create_metalog(Origin::signed(20), did_claimed , 0, did_new.clone()), ERR_DID_ALREADY_CLAIMED);
			assert_noop!(Metalog::create_metalog(Origin::signed(20), did_too_long.clone(), 0, did_new.clone()), ERR_BYTEARRAY_LIMIT_DID);
			assert_noop!(Metalog::create_metalog(Origin::signed(20), did_new.clone(), 0, location_too_long.clone()), ERR_BYTEARRAY_LIMIT_LOCATION);
			assert_ok!(Metalog::create_metalog(Origin::signed(20), did_new.clone(), 0, did_new.clone()));			
			assert_eq!(Metalog::owner_of_did(did_new), Some(20));
		});
	}

	#[test]
	fn transfer_ownership_works() {
		let did_claimed = vec![1,2,3];
		let did_new = vec![1, 2, 3, 4];
		with_externalities(&mut new_test_ext(), || {
			assert_noop!(Metalog::transfer_ownership(Origin::signed(0),2, did_new), ERR_DID_NOT_EXIST);
			assert_noop!(Metalog::transfer_ownership(Origin::signed(1),2, did_claimed.clone()), ERR_NOT_OWNER);
			assert_ok!(Metalog::transfer_ownership(Origin::signed(0),20, did_claimed.clone()));	
			assert_eq!(Metalog::owner_of_did(did_claimed), Some(20));
		});
	}

	#[test]
	fn buy_unique_name_works() {
		let did_claimed = vec![1,2,3];
		let did_new = vec![1, 2, 3, 4];
		let un = vec![1];
		let mut un_too_long = un.clone();
		for _i in 1..60 {
			un_too_long.push(2);
		}	
		with_externalities(&mut new_test_ext(), || {
			assert_noop!(Metalog::buy_unique_name(Origin::signed(0), did_new, un.clone()), ERR_DID_NOT_EXIST);
			assert_noop!(Metalog::buy_unique_name(Origin::signed(1), did_claimed.clone(), un.clone()), ERR_NOT_OWNER);
			assert_noop!(Metalog::buy_unique_name(Origin::signed(0), did_claimed.clone(), un.clone()), "balance too low to send value");
			let _ = Balances::make_free_balance_be(&0, 5000);
			assert_noop!(Metalog::buy_unique_name(Origin::signed(0), did_claimed.clone(), un_too_long), ERR_BYTEARRAY_LIMIT_NAME);
			assert_ok!(Metalog::buy_unique_name(Origin::signed(0), did_claimed.clone(), un.clone()));
			let metadata = Metalog::meta_of_did(&did_claimed);
			assert_eq!(metadata.unique_name, un.clone());
		});
	}

	#[test]
	fn change_license_code_works() {
		let did_claimed = vec![1,2,3];
		let did_new = vec![1, 2, 3, 4];
		with_externalities(&mut new_test_ext(), || {
			assert_noop!(Metalog::change_license_code(Origin::signed(0), did_new, 1), ERR_DID_NOT_EXIST);
			assert_noop!(Metalog::change_license_code(Origin::signed(1), did_claimed.clone(), 1), ERR_NOT_OWNER);
			assert_ok!(Metalog::change_license_code(Origin::signed(0), did_claimed.clone(), 4));
			let metadata = Metalog::meta_of_did(&did_claimed);
			assert_eq!(metadata.license_code, 4);
		});
	}

	#[test]
	fn change_storage_location_works() {
		let did_claimed = vec![1,2,3];
		let did_new = vec![1, 2, 3, 4];
		let location = vec![0,1];
		let mut location_too_long = location.clone();
		for _i in 1..100 {
			location_too_long.push(1);
		}
		with_externalities(&mut new_test_ext(), || {
			assert_noop!(Metalog::change_storage_location(Origin::signed(0), did_new, location.clone()), ERR_DID_NOT_EXIST);
			assert_noop!(Metalog::change_storage_location(Origin::signed(1), did_claimed.clone(), location.clone()), ERR_NOT_OWNER);
			assert_noop!(Metalog::change_storage_location(Origin::signed(0), did_claimed.clone(), location_too_long.clone()), ERR_BYTEARRAY_LIMIT_LOCATION);
			assert_ok!(Metalog::change_storage_location(Origin::signed(0), did_claimed.clone(), location.clone()));
			let metadata = Metalog::meta_of_did(&did_claimed);
			assert_eq!(metadata.storage_location, location.clone());
		});
	}
}

