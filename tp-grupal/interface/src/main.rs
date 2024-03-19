mod account;
use account::Account;
use btc_node::{
    api::{NodeApi, WalletApi},
    bitcoin_node::Node,
    blockchain::txs::Tx,
    config::Config,
    protocol_error::ProtocolError,
    utils::bytes_to_hex_string,
};
use glib::Receiver;
use gtk::{
    ffi::{GTK_MESSAGE_INFO, GTK_MESSAGE_WARNING},
    prelude::*,
    Builder, Button, ComboBoxText, Entry, Label, ListStore, ProgressBar, SpinButton, Stack,
    ToggleButton,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    env,
    rc::Rc,
    sync::mpsc::{self, Sender},
};

fn main() -> Result<(), ProtocolError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(ProtocolError::Error(
            "Incorrect amount of arguments were given. Need 1".to_string(),
        ));
    }

    let (sender, receiver) = glib::MainContext::channel::<NodeApi>(glib::PRIORITY_DEFAULT);
    let (tx, rx) = mpsc::channel();

    let node_thread = std::thread::spawn(move || -> Result<(), ProtocolError> {
        let config = Config::new(&args[1])?;
        let mut my_node = Node::new(config, sender)?;
        my_node.initialize()?;
        my_node.listen(rx)?;
        Ok(())
    });

    init(receiver, tx);

    node_thread.join().unwrap()?;

    Ok(())
}

fn init(receiver: Receiver<NodeApi>, sender: Sender<WalletApi>) {
    let accounts: Rc<RefCell<HashMap<String, Account>>> = Rc::new(RefCell::new(HashMap::new()));

    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let glade_src = include_str!("interface.glade");
    let builder = Builder::from_string(glade_src);

    let window: gtk::Window = builder.object("app").expect("Failed to get window");
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        std::process::exit(0);
    });

    set_all_menus(&builder);
    create_account_button_on_clicked(&builder, sender.clone(), &accounts);
    pay_button_on_clicked(&builder, &accounts, sender);
    combo_box_on_changed(&builder, &accounts);
    set_necesary_widgets_during_block_download(&builder);

    attach(receiver, &accounts, &builder);
    window.show_all();
    gtk::main();
}

fn set_necesary_widgets_during_block_download(builder: &Builder) {
    deactivate_necesary_buttons_during_block_download(&builder);
    set_spinner_to(builder, true);
}

fn set_spinner_to(builder: &Builder, active: bool) {
    let overview_spinner: gtk::Spinner = builder
        .object("overview_page_spinner")
        .expect("Failed retrieving overview page spinner");

    let send_spinner: gtk::Spinner = builder
        .object("send_page_spinner")
        .expect("Failed retrieving send page spinner");

    let accounts_spinner: gtk::Spinner = builder
        .object("accounts_page_spinner")
        .expect("Failed retrieving accounts page spinner");

    let transactions_spinner: gtk::Spinner = builder
        .object("transactions_page_spinner")
        .expect("Failed retrieving transactions page spinner");

    overview_spinner.set_active(active);
    send_spinner.set_active(active);
    accounts_spinner.set_active(active);
    transactions_spinner.set_active(active);
}

fn deactivate_necesary_buttons_during_block_download(builder: &Builder) {
    let create_account_button: Button = builder
        .object("accounts_page_frame1_button")
        .expect("Failed retrieving create account button");

    let pay_button: Button = builder
        .object("pay_button")
        .expect("Failed retrieving pay button");

    create_account_button.set_sensitive(false);
    pay_button.set_sensitive(false);
}

fn combo_box_on_changed(builder: &Builder, accounts: &Rc<RefCell<HashMap<String, Account>>>) {
    let accounts_clone = Rc::clone(accounts);
    let builder_clone = builder.clone();

    let combo_box: ComboBoxText = builder
        .object("wallets_combo_box")
        .expect("Failed to get combobox");

    combo_box.connect_changed(move |combo_box| {
        if let Some(current_account) = combo_box.active_text() {
            for (_address, account) in accounts_clone.borrow_mut().iter() {
                let name = account.name.clone();
                if name == current_account {
                    actualize_balance_label(&builder_clone, account.balance);
                    actualize_pending_balance_label(&builder_clone, account.pending_balance);
                    actualize_total_balance(
                        &builder_clone,
                        account.balance,
                        account.pending_balance,
                    );

                    re_set_pending_transactions(&builder_clone, &account.pending_tx);

                    re_set_transactions(&builder_clone, &account.transactions);
                }
            }
        }
    });
}

fn re_set_transactions(builder: &Builder, transactions: &Vec<Tx>) {
    let transactions_list_store: ListStore = builder
        .object("transactions_columns")
        .expect("Failed to retrieve transactions list store");

    transactions_list_store.clear();
    set_transactions(&transactions, &transactions_list_store);
}

fn re_set_pending_transactions(
    builder: &Builder,
    pending_tx: &HashMap<[u8; 32], (Tx, i64, String, String)>,
) {
    let pending_transactions_list_store: ListStore = builder
        .object("pending_transactions")
        .expect("Failed to retrieve pending transactions list store");

    pending_transactions_list_store.clear();
    set_pending_transactions(&pending_tx, &pending_transactions_list_store);
}

fn actualize_total_balance(builder: &Builder, balance: i64, pending_balance: i64) {
    let total_balance_label: Label = builder
        .object("total_size_label")
        .expect("Failed to get total balance label");

    let total_balance = balance + pending_balance;
    total_balance_label.set_text(&total_balance.to_string());
}

fn actualize_pending_balance_label(builder: &Builder, pending_balance: i64) {
    let pending_balance_label: Label = builder
        .object("pending_row_size")
        .expect("Failed to get pending balance label");

    pending_balance_label.set_text(&pending_balance.to_string());
}

fn actualize_balance_label(builder: &Builder, balance: i64) {
    let balance_label: Label = builder
        .object("available_row_size")
        .expect("Failed to get balance label");

    balance_label.set_text(&balance.to_string());
}

fn pay_button_on_clicked(
    builder: &Builder,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
    sender: Sender<WalletApi>,
) {
    let accounts_clone = Rc::clone(accounts);

    let pay_button: Button = builder
        .object("pay_button")
        .expect("Failed to retrieve pay button.");

    let pay_entry: Entry = builder
        .object("pay_to_entry")
        .expect("Failed to retrieve pay entry");

    let amount_spin_button: SpinButton = builder
        .object("amount_spin_button")
        .expect("Failed to retrieve name entry");

    let fee_amount_spin_button: SpinButton = builder
        .object("fee_amount_spin_button")
        .expect("Failed to retrieve name entry");

    let wallets_combo_box: ComboBoxText = builder
        .object::<ComboBoxText>("wallets_combo_box")
        .expect("Failed to get wallet combobox");

    pay_button.connect_clicked(move |_pay_button| {
        if validate_text_is_not_empty(&pay_entry, "Addres to pay to is missing") {
            let address_to_pay = pay_entry.text().to_string();
            let fee_amount = fee_amount_spin_button.value_as_int() as i64;
            let amount_to_pay = amount_spin_button.value_as_int() as i64;

            let mut wif: String = "".to_string();

            for (_address, account) in accounts_clone.borrow_mut().iter() {
                if let Some(text) = wallets_combo_box.active_text() {
                    if account.name == text {
                        wif = account.wif.clone();
                    }
                }
            }

            if !wif.is_empty() {
                sender
                    .send(WalletApi::PayTo(
                        wif,
                        address_to_pay,
                        amount_to_pay,
                        fee_amount,
                    ))
                    .unwrap();

                pay_entry.set_text("");
                fee_amount_spin_button.set_value(0 as f64);
                amount_spin_button.set_value(0 as f64);
            } else {
                create_notification_window(
                    gtk::MessageType::__Unknown(GTK_MESSAGE_WARNING),
                    "Warning",
                    "You have to select or log an account first to pay",
                );
            }
        }
    });
}

fn set_transactions(transactions: &Vec<Tx>, transactions_table: &gtk::ListStore) {
    for tx in transactions {
        let txid = btc_node::utils::bytes_to_hex_string(&tx.tx_id);
        let data_for_column_1 = txid.to_value();
        let data_for_column_2 = tx.get_tx_value().to_value();
        let data_for_column_3 = (tx.tx_out.len() as u32).to_value();
        let data_for_column_4 = (tx.tx_out.len() as u32).to_value();

        let array_of_data: &[(u32, &dyn ToValue)] = &[
            (0, &data_for_column_1),
            (1, &data_for_column_2),
            (2, &data_for_column_3),
            (3, &data_for_column_4),
        ];
        transactions_table.insert_with_values(None, array_of_data);
    }
}

fn _set_transaction(
    tx: Tx,
    payer_address: String,
    payee_address: String,
    transactions_table: &gtk::ListStore,
) {
    let txid = btc_node::utils::bytes_to_hex_string(&tx.tx_id);
    let data_for_column_1 = txid.to_value();
    let data_for_column_2 = tx.value_payed_to_address(&payer_address).to_value();
    let data_for_column_3 = payer_address.to_value();
    let data_for_column_4 = payee_address.to_value();

    let array_of_data: &[(u32, &dyn ToValue)] = &[
        (0, &data_for_column_1),
        (1, &data_for_column_2),
        (2, &data_for_column_3),
        (3, &data_for_column_4),
    ];
    transactions_table.insert_with_values(Some(1), array_of_data);
}

fn set_pending_transactions(
    pending_tx: &HashMap<[u8; 32], (Tx, i64, String, String)>,
    pending_transactions_table: &ListStore,
) {
    for (tx, amount, payer, payee) in pending_tx.values() {
        let txid = btc_node::utils::bytes_to_hex_string(&tx.tx_id);
        let data_for_column_1 = txid.to_value();
        let data_for_column_2 = amount;
        let data_for_column_3 = payer;
        let data_for_column_4 = payee;

        let array_of_data: &[(u32, &dyn ToValue)] = &[
            (0, &data_for_column_1),
            (1, &data_for_column_2),
            (2, &data_for_column_3),
            (3, &data_for_column_4),
        ];
        pending_transactions_table.insert_with_values(None, array_of_data);
    }
}

fn set_all_menus(builder: &Builder) {
    let stack: Rc<RefCell<Stack>> = Rc::new(RefCell::new(
        builder.object("stack").expect("Failed to get stack"),
    ));

    let button_overview: ToggleButton = builder
        .object("menu_button_overview")
        .expect("Failed to get overviews button");
    let button_send: ToggleButton = builder
        .object("menu_button_send")
        .expect("Failed to get send button");
    let button_accounts: ToggleButton = builder
        .object("menu_button_accounts")
        .expect("Failed to get account button");
    let button_transactions: ToggleButton = builder
        .object("menu_button_transactions")
        .expect("Failed to get transactions button");

    set_menu(
        &stack,
        &button_overview,
        &button_accounts,
        &button_send,
        &button_transactions,
        "overview_page".to_string(),
    );
    set_menu(
        &stack,
        &button_accounts,
        &button_overview,
        &button_send,
        &button_transactions,
        "accounts_page".to_string(),
    );
    set_menu(
        &stack,
        &button_send,
        &button_accounts,
        &button_overview,
        &button_transactions,
        "send_page".to_string(),
    );
    set_menu(
        &stack,
        &button_transactions,
        &button_accounts,
        &button_overview,
        &button_send,
        "transactions_page".to_string(),
    );
}

fn create_notification_window(notification_type: gtk::MessageType, title: &str, message: &str) {
    let glade_src = include_str!("interface.glade");
    let builder = Builder::from_string(glade_src);
    let parent: gtk::Window = builder.object("app").expect("Failed to get window");

    let dialog = gtk::MessageDialog::new(
        Some(&parent),
        gtk::DialogFlags::empty(),
        notification_type,
        gtk::ButtonsType::Ok,
        "",
    );

    dialog.set_transient_for(Some(&parent));
    dialog.set_position(gtk::WindowPosition::CenterOnParent);
    dialog.set_text(Some(title));
    dialog.set_secondary_text(Some(message));

    dialog.connect_response(|dialog, _| dialog.close());
    dialog.run();
}

fn validate_account_creation_info(
    name_entry: &Entry,
    address_entry: &Entry,
    private_key_entry: &Entry,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
) -> bool {
    if !validate_text_is_not_empty(name_entry, "Name is missing") {
        return false;
    };
    if !validate_text_is_not_empty(address_entry, "Adress is missing") {
        return false;
    };
    if !validate_text_is_not_empty(private_key_entry, "Private key is missing") {
        return false;
    };

    if !validate_account_not_already_logged_in(address_entry, accounts) {
        return false;
    };
    if !validate_name_is_unused(accounts, name_entry) {
        return false;
    };

    true
}

fn validate_name_is_unused(
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
    name_entry: &Entry,
) -> bool {
    let accounts_clone = Rc::clone(accounts);
    let names: Vec<String> = accounts_clone
        .borrow_mut()
        .values()
        .map(|v| v.name.clone())
        .collect();

    if names.contains(&name_entry.text().to_string()) {
        create_notification_window(
            gtk::MessageType::__Unknown(GTK_MESSAGE_WARNING),
            "Warning",
            "Account name is already used, pick another one",
        );

        return false;
    }

    true
}

fn validate_account_not_already_logged_in(
    address_entry: &Entry,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
) -> bool {
    let accounts_clone = Rc::clone(accounts);

    if accounts_clone
        .borrow_mut()
        .contains_key(&address_entry.text().to_string())
    {
        create_notification_window(
            gtk::MessageType::__Unknown(GTK_MESSAGE_WARNING),
            "Warning",
            "Account is already logged in",
        );

        return false;
    }

    true
}

fn validate_text_is_not_empty(text: &Entry, message: &str) -> bool {
    if text.text() == "" {
        create_notification_window(
            gtk::MessageType::__Unknown(GTK_MESSAGE_WARNING),
            "Warning",
            message,
        );

        return false;
    }
    true
}

fn create_account_button_on_clicked(
    builder: &Builder,
    sender: Sender<WalletApi>,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
) {
    let create_account_button: Button = builder
        .object("accounts_page_frame1_button")
        .expect("Failed to retrieve create button.");

    let accounts_clone = Rc::clone(accounts);

    let combo_box_wallets: ComboBoxText = builder
        .object::<ComboBoxText>("wallets_combo_box")
        .expect("Failed to get wallet combobox");

    let name_entry: Entry = builder
        .object("name_row_entry")
        .expect("Failed to retrieve name entry");
    let private_key_entry: Entry = builder
        .object("private_key_row_entry")
        .expect("Failed to retrieve private key entry");
    let address_entry: Entry = builder
        .object("public_key_row_entry")
        .expect("Failed to retrieve public key entry");

    create_account_button.connect_clicked(move |_button| {
        if validate_account_creation_info(
            &name_entry,
            &address_entry,
            &private_key_entry,
            &accounts_clone,
        ) {
            combo_box_wallets.append_text(name_entry.text().as_str());

            let address = address_entry.text().to_string();

            let new_account = Account::new(
                address_entry.text().to_string(),
                private_key_entry.text().to_string(),
                0,
                name_entry.text().to_string(),
            );
            accounts_clone
                .borrow_mut()
                .insert(address.clone(), new_account);

            sender.send(WalletApi::AddAddress(address)).unwrap();

            let index = combo_box_wallets.model().unwrap().iter_n_children(None) - 1;
            combo_box_wallets.set_active(Some(index as u32));

            name_entry.set_text("");
            address_entry.set_text("");
            private_key_entry.set_text("");
        }
    });
}

fn attach(
    receiver: Receiver<NodeApi>,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
    builder: &Builder,
) {
    let builder_clone = builder.clone();
    let accounts_clone = Rc::clone(accounts);

    receiver.attach(None, move |msg| {
        let accounts_clone = Rc::clone(&accounts_clone);

        match msg {
            NodeApi::NodeReady => handle_node_ready_message(&builder_clone),
            NodeApi::NewTx(tx, payer_addr, addr) => {
                handle_new_tx_message(&builder_clone, &accounts_clone, addr, tx, payer_addr)
            }
            NodeApi::ConfirmedTx(txid, addr) => {
                handle_confirmed_tx_message(&builder_clone, &accounts_clone, addr, txid)
            }
            NodeApi::Balance(balance, addr) => {
                handle_balance_message(&builder_clone, &accounts_clone, addr, balance)
            }
            NodeApi::AddPendingBalance(pending_balance, addr) => {
                handle_add_pending_balance_message(
                    &builder_clone,
                    &accounts_clone,
                    addr,
                    pending_balance,
                )
            }
            NodeApi::AddConfirmedBalance(confirmed_balance, addr) => {
                handle_add_confirmed_balance_message(
                    &builder_clone,
                    &accounts_clone,
                    addr,
                    confirmed_balance,
                )
            }
            NodeApi::PaymentConfirmation(tx, payer_address, payee_address, amount) => {
                handle_payment_confirmation_message(
                    &builder_clone,
                    &accounts_clone,
                    tx,
                    payer_address,
                    payee_address,
                    amount,
                )
            }
            NodeApi::History(txs, addr) => {
                handle_history_message(&builder_clone, &accounts_clone, txs, addr)
            }
            NodeApi::Error(error) => create_notification_window(
                gtk::MessageType::__Unknown(GTK_MESSAGE_WARNING),
                "Warning",
                &format!("{}", error),
            ),
            NodeApi::Loading(progress) => handle_loading_message(&builder_clone, progress),
            NodeApi::FinishedConnectingToPeers => {
                handle_finished_connecting_to_peers_message(&builder_clone)
            }
        }
        glib::Continue(true)
    });
}

fn handle_finished_connecting_to_peers_message(builder: &Builder) {
    let overview_page_label: Label = builder
        .object("overview_page_progress_bar_label")
        .expect("Failed to get overview page progress bar label");

    let send_page_label: Label = builder
        .object("send_page_progress_bar_label")
        .expect("Failed to get send page progress bar label");

    let transactions_page_label: Label = builder
        .object("transactions_page_progress_bar_label")
        .expect("Failed to get transactions page progress bar label");

    let accounts_page_label: Label = builder
        .object("accounts_page_progress_bar_label")
        .expect("Failed to get accounts page progress bar label");

    overview_page_label.set_text("Downloading Blocks...");
    send_page_label.set_text("Downloading Blocks...");
    transactions_page_label.set_text("Downloading Blocks...");
    accounts_page_label.set_text("Downloading Blocks...");
}

fn handle_loading_message(builder: &Builder, progress: f64) {
    let overview_prog_bar: ProgressBar = builder
        .object("overview_page_progress_bar")
        .expect("Failed to get overview page progressbar");

    let send_prog_bar: ProgressBar = builder
        .object("send_page_progress_bar")
        .expect("Failed to get send page progressbar");

    let transactions_prog_bar: ProgressBar = builder
        .object("transactions_page_progress_bar")
        .expect("Failed to get transactions page progressbar");

    let accounts_prog_bar: ProgressBar = builder
        .object("accounts_page_progress_bar")
        .expect("Failed to get accounts page progressbar");

    overview_prog_bar.set_fraction(progress);
    send_prog_bar.set_fraction(progress);
    transactions_prog_bar.set_fraction(progress);
    accounts_prog_bar.set_fraction(progress);
}

fn handle_history_message(
    builder: &Builder,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
    txs: Vec<Tx>,
    addr: String,
) {
    let transactions_table: gtk::ListStore = builder
        .object("transactions_columns")
        .expect("Failed retrieving transaction table");

    if let Some(account) = accounts.borrow_mut().get_mut(&addr) {
        (*account).transactions.extend_from_slice(&txs[..]);
        transactions_table.clear();
        set_transactions(&txs, &transactions_table);
    }
}

fn handle_payment_confirmation_message(
    builder: &Builder,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
    tx: Tx,
    payer_address: String,
    payee_address: String,
    amount: i64,
) {
    let pending_transactions_table: gtk::ListStore = builder
        .object("pending_transactions")
        .expect("Failed retrieving pending transaction table");

    if let Some(account) = accounts.borrow_mut().get_mut(&payer_address) {
        (account).pending_tx.insert(
            tx.tx_id,
            (
                tx.clone(),
                tx.value_payed_to_address(&payee_address),
                payer_address,
                payee_address,
            ),
        );

        create_notification_window(
            gtk::MessageType::__Unknown(GTK_MESSAGE_INFO),
            "Succesful Payment",
            "Payment correctly sent",
        );

        account.balance -= amount;
        actualize_balance_label(builder, account.balance);
        actualize_pending_balance_label(builder, account.pending_balance);

        pending_transactions_table.clear();
        set_pending_transactions(&(*account).pending_tx, &pending_transactions_table);
    }
}

fn handle_add_confirmed_balance_message(
    builder: &Builder,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
    addr: String,
    confirmed_balance: i64,
) {
    let pending_btc_label: Label = builder
        .object("pending_row_size")
        .expect("Failed retrieving avaliable btc label");

    let available_btc_label: Label = builder
        .object("available_row_size")
        .expect("Failed retrieving avaliable btc label");

    let total_btc_label: Label = builder
        .object("total_size_label")
        .expect("Failed retrieving avaliable btc label");

    if let Some(account) = accounts.borrow_mut().get_mut(&addr) {
        (*account).pending_balance -= confirmed_balance;
        (*account).balance += confirmed_balance;
        let total = account.pending_balance + account.balance;

        pending_btc_label.set_text(&(account).pending_balance.to_string());
        available_btc_label.set_text(&(account).balance.to_string());
        total_btc_label.set_text(&total.to_string());
    }
}

fn handle_add_pending_balance_message(
    builder: &Builder,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
    addr: String,
    pending_balance: i64,
) {
    let balance_btc_label: Label = builder
        .object("available_row_size")
        .expect("Failed to get balance label");

    let pending_btc_label: Label = builder
        .object("pending_row_size")
        .expect("Failed retrieving avaliable btc label");

    let total_btc_label: Label = builder
        .object("total_size_label")
        .expect("Failed retrieving avaliable btc label");

    if let Some(account) = accounts.borrow_mut().get_mut(&addr) {
        (account).pending_balance += pending_balance;
        (account).balance -= pending_balance;
        let total = (account).balance + pending_balance;

        pending_btc_label.set_text(&account.pending_balance.to_string());
        balance_btc_label.set_text(&account.balance.to_string());
        total_btc_label.set_text(&total.to_string());
    }
}

fn handle_balance_message(
    builder: &Builder,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
    addr: String,
    balance: i64,
) {
    let available_btc_label: Label = builder
        .object("available_row_size")
        .expect("Failed retrieving avaliable btc label");

    let total_btc_label: Label = builder
        .object("total_size_label")
        .expect("Failed retrieving avaliable btc label");

    if let Some(account) = accounts.borrow_mut().get_mut(&addr) {
        (account).balance = balance;
        available_btc_label.set_text(&balance.to_string());
        let total = balance + (account).pending_balance;
        total_btc_label.set_text(&total.to_string());
    }
}

fn handle_confirmed_tx_message(
    builder: &Builder,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
    addr: String,
    txid: [u8; 32],
) {
    let transactions_table: gtk::ListStore = builder
        .object("transactions_columns")
        .expect("Failed retrieving transaction table");

    let pending_transactions_table: gtk::ListStore = builder
        .object("pending_transactions")
        .expect("Failed retrieving pending transaction table");

    create_notification_window(
        gtk::MessageType::__Unknown(GTK_MESSAGE_INFO),
        "One pending transaction is now confirmed.",
        &format!("TXID: {}", bytes_to_hex_string(&txid)),
    );

    if let Some(account) = accounts.borrow_mut().get_mut(&addr) {
        if let Some((tx, _, _, _)) = (account).pending_tx.remove(&txid) {
            (account).transactions.push(tx);
            transactions_table.clear();
            set_transactions(&(account).transactions, &transactions_table);

            pending_transactions_table.clear();
            set_pending_transactions(&(account).pending_tx, &pending_transactions_table);
        };
    }
}

fn handle_new_tx_message(
    builder: &Builder,
    accounts: &Rc<RefCell<HashMap<String, Account>>>,
    addr: String,
    tx: Tx,
    payer_addr: String,
) {
    let accounts_clone = Rc::clone(accounts);

    let pending_transactions_table: gtk::ListStore = builder
        .object("pending_transactions")
        .expect("Failed retrieving pending transaction table");

    if let Some(account) = accounts_clone.borrow_mut().get_mut(&addr) {
        create_notification_window(
            gtk::MessageType::__Unknown(GTK_MESSAGE_INFO),
            "A new transaction related to your account has arrived",
            &format!(
                "Tx ID:{} '\n' Amount {} satoshi ",
                bytes_to_hex_string(&tx.tx_id),
                tx.get_tx_value()
            ),
        );
        (account).pending_tx.insert(
            tx.tx_id,
            (
                tx.clone(),
                tx.value_payed_to_address(&addr),
                payer_addr,
                addr,
            ),
        );

        pending_transactions_table.clear();
        set_pending_transactions(&(*account).pending_tx, &pending_transactions_table);
    };
}

fn handle_node_ready_message(builder: &Builder) {
    let create_account_button: Button = builder
        .object("accounts_page_frame1_button")
        .expect("Failed retrieving create account button");

    let pay_button: Button = builder
        .object("pay_button")
        .expect("Failed retrieving pay button");

    create_account_button.set_sensitive(true);
    pay_button.set_sensitive(true);
    set_spinner_to(builder, false);

    set_all_downloading_blocks_labels_to(&builder, "Finished download!");
    handle_loading_message(&builder, 1 as f64); //Makes all progress bars look full

    create_notification_window(
        gtk::MessageType::__Unknown(GTK_MESSAGE_INFO),
        "Finished downloading blocks",
        "The wallet is ready to be used",
    );
}

fn set_all_downloading_blocks_labels_to(builder: &Builder, text: &str) {
    let overview_label: Label = builder
        .object("overview_page_progress_bar_label")
        .expect("Failed retrieving overview page progress bar label");

    let send_label: Label = builder
        .object("send_page_progress_bar_label")
        .expect("Failed retrieving send page progress bar label");

    let transactions_label: Label = builder
        .object("transactions_page_progress_bar_label")
        .expect("Failed retrieving transactions page progress bar label");

    let accounts_label: Label = builder
        .object("accounts_page_progress_bar_label")
        .expect("Failed retrieving accounts page progress bar label");

    overview_label.set_text(text);
    send_label.set_text(text);
    transactions_label.set_text(text);
    accounts_label.set_text(text);
}

fn set_menu(
    stack: &Rc<RefCell<gtk::Stack>>,
    active: &gtk::ToggleButton,
    other1: &gtk::ToggleButton,
    other2: &gtk::ToggleButton,
    other3: &gtk::ToggleButton,
    page_name: String,
) {
    let other1 = other1.clone();
    let other2 = other2.clone();
    let other3 = other3.clone();

    let stack_clone = stack.clone();

    let default_page = "default_page".to_string();

    let current_page = Rc::new(RefCell::new(default_page.clone()));
    let current_page_clone = current_page.clone();
    let page_name = page_name.clone();

    active.connect_toggled(move |toggle_button| {
        if toggle_button.is_active() {
            other1.set_active(false);
            other2.set_active(false);
            other3.set_active(false);
        }

        let stack = stack_clone.borrow_mut();
        let mut current_page = current_page_clone.borrow_mut();

        if *current_page != page_name.to_string() {
            stack.set_visible_child_name(&page_name);
            *current_page = page_name.clone();
        } else {
            stack.set_visible_child_name("default_page");
            *current_page = "default_page".to_string();
        }
    });
}
