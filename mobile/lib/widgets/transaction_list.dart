import 'package:flutter/material.dart';

import 'package:collection/collection.dart';

import 'package:intl/intl.dart';

import 'package:flouze_flutter/flouze_flutter.dart';

import 'package:flouze/localization.dart';
import 'package:flouze/utils/account_members.dart';
import 'package:flouze/utils/amounts.dart';
import 'package:flouze/utils/config.dart';
import 'package:flouze/utils/keys.dart';

class TransactionList extends StatelessWidget {
  final Function listEquality = ListEquality().equals;
  final DateFormat dateFormat = DateFormat.yMMMd();

  final Iterable<Transaction> transactions;
  final List<Person> members;
  final void Function(Transaction) onTap;

  TransactionList({Key key, @required this.transactions, @required this.members, this.onTap}) : super(key: key);

  String formatPayedBy(List<PayedBy> payedBys) {
    final names = payedBys.where((p) => p.amount > 0).map((p) => personName(members, p.person)).toList();
    names.sort();
    return names.join(', ');
  }

  @override
  Widget build(BuildContext context) {
    int transactionIndex = -1;
    final List<Widget> transactionWidgets = (transactions ?? []).map((tx) {
      final String date = dateFormat.format(DateTime.fromMillisecondsSinceEpoch(1000*tx.timestamp.toInt()));
      final String payedBy = formatPayedBy(tx.payedBy);

      transactionIndex++;

      return ListTile(
        key: subkey(key, '-$transactionIndex'),
        title: Row(
          children: <Widget>[
            Expanded(child: Text(tx.label, key: subkey(key, '-$transactionIndex-label'))),
            Text('${amountToString(tx.amount)} ${AppConfig.currencySymbol}', key: subkey(key, '-$transactionIndex-amount'))
          ],
        ),
        subtitle: Text(FlouzeLocalizations.of(context).transactionListOnBy(date, payedBy)),
        onTap: () {
          if (onTap != null) {
            onTap(tx);
          }
        },
      );
    }).toList();

    return ListView(
      shrinkWrap: false,
      children: [
        ...transactionWidgets,
        ListTile(
          enabled: false,
          title: Container(height: 32,)
        )
      ],
    );
  }
}
