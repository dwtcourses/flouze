import 'package:rxdart/rxdart.dart';

import 'package:share/share.dart';

import 'package:flouze_flutter/flouze_flutter.dart' as Flouze;

import 'package:flouze/utils/account_config.dart';
import 'package:flouze/utils/account_config_store.dart' as AccountConfigStore;
import 'package:flouze/utils/rpc_client.dart' as RpcClient;
import 'package:flouze/utils/services.dart';

enum AccountSyncError {
  LoadAccountConfigError,
  SaveAccountConfigError,
  ShareError,
  SynchronizationError,
}

class AccountSyncBloc {
  AccountSyncBloc();

  final _syncController = BehaviorSubject<AccountSyncState>();

  void loadAccountConfig(List<int> accountUuid) {
    _syncController.add(AccountSyncLoadingState());
    AccountConfigStore.loadAccountConfig(accountUuid)
        .then((accountConfig) {
          _syncController.add(AccountSyncLoadedState(accountConfig));
        })
        .catchError((e) {
          _syncController.add(AccountSyncErrorState(AccountSyncError.LoadAccountConfigError, e.toString()));
        });
  }

  Future<AccountConfig> _ensureRemoteAccountExists(Flouze.Account account, AccountConfig config) {
    if (config.synchronized) {
      return Future.value(config);
    }

    final newAccountConfig = config.rebuild((b) => b..synchronized = true);

    return RpcClient.getJsonRpcClient()
        .then((client) => client.createAccount(account))
        .then((_) => AccountConfigStore.saveAccountConfig(account.uuid, newAccountConfig))
        .then((_) => newAccountConfig);
  }

  AccountConfig _getLoadedAccountConfig() =>
      (_syncController.value.runtimeType == AccountSyncLoadedState) ?
        (_syncController.value as AccountSyncLoadedState).accountConfig : null;

  void share(Flouze.Account account, String Function(String uri) shareMessage) {
    final AccountConfig config = _getLoadedAccountConfig();

    if (config == null) {
      // Silently ignore those requests
      return;
    }

    _syncHelper(account, config)
      .then((_) {
        return shareAccountUri(account.uuid)
          .then((uri) {
            Share.share(shareMessage(uri));
          })
          .catchError((e) {
            _syncController.add(AccountSyncErrorState(AccountSyncError.ShareError, e.toString()));
            _syncController.add(AccountSyncLoadedState(config));
          });
      }).catchError((_) {});
  }

  void _setMeUuid(Flouze.Account account, List<int> uuid) {
    final AccountSyncState state = _syncController.value;

    if (uuid == null || uuid.isEmpty) {
      return;
    }

    final AccountConfig config  = (state as AccountSyncLoadedState).accountConfig;
    final newAccountConfig = config.rebuild((b) => b..meUuid.update((b) => b..clear()..addAll(uuid)));

    AccountConfigStore.saveAccountConfig(account.uuid, newAccountConfig)
      .then((_) => _syncController.add(AccountSyncLoadedState(newAccountConfig)))
      .catchError((e) {
        _syncController.add(AccountSyncErrorState(AccountSyncError.SaveAccountConfigError, e.toString()));
        _syncController.add(AccountSyncLoadedState(config));
      });
  }

  Future<void> _syncHelper(Flouze.Account account, AccountConfig config) {
    _syncController.add(AccountSyncSynchronizingState(config));

    return _ensureRemoteAccountExists(account, config)
      .then((newConfig) {
        return Future.wait([RpcClient.getJsonRpcClient(), getRepository()])
          .then((ctx) {
            final Flouze.JsonRpcClient client = ctx[0];
            final Flouze.SledRepository repository = ctx[1];

            return Flouze.Sync.sync(repository, client, account.uuid).then((_) => repository.flush());
        }).then((_) => newConfig);
      })
      .then((newConfig) {
        _syncController.add(AccountSyncLoadedState(newConfig));
      })
      .catchError((e) {
        _syncController.add(AccountSyncErrorState(AccountSyncError.SynchronizationError, e.toString()));
        _syncController.add(AccountSyncLoadedState(config));
        return Future.error(e);
      });
  }

  void synchronize(Flouze.Account account) {
    final AccountConfig config = _getLoadedAccountConfig();

    if (config == null) {
      // Silently ignore those requests
      return;
    }

    // Swallow the error here, _syncHelper already dispatched an error state
    _syncHelper(account, config).catchError((_) {});
  }

  Stream<AccountSyncState> get sync => _syncController.stream;

  void dispose() {
    _syncController.close();
  }
}

class AccountSyncState {}

// Loading the account configuration from disk
class AccountSyncLoadingState extends AccountSyncState {}

class AccountSyncLoadedState extends AccountSyncState {
  AccountSyncLoadedState(this.accountConfig);
  final AccountConfig accountConfig;
}

class AccountSyncSynchronizingState extends AccountSyncLoadedState {
  AccountSyncSynchronizingState(AccountConfig accountConfig) : super(accountConfig);
}

class AccountSyncErrorState extends AccountSyncState {
  AccountSyncErrorState(this.errorKind, this.message);
  final AccountSyncError errorKind;
  final String message;
}
