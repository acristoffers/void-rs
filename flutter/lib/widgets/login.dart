// The MIT License (MIT)
//
// Copyright (c) 2021 Álan Crístoffer e Sousa
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
//   "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
// SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:void_gui/widgets/filepicker.dart';

class LoginPage extends StatefulWidget {
  LoginPage({Key? key}) : super(key: key);

  @override
  _LoginPageState createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  final storeController = TextEditingController();
  final passwordController = TextEditingController();
  final focusNode = FocusNode();
  final focusNode2 = FocusNode();
  bool? isCreate;

  @override
  Widget build(BuildContext context) {
    final node = this.isCreate != null ? focusNode2 : focusNode;
    FocusScope.of(context).requestFocus(node);
    return Shortcuts(
      shortcuts: Platform.isMacOS
          ? <SingleActivator, Intent>{
              SingleActivator(LogicalKeyboardKey.keyO, meta: true):
                  OpenIntent(),
              SingleActivator(LogicalKeyboardKey.keyN, meta: true):
                  CreateIntent(),
              SingleActivator(LogicalKeyboardKey.escape): CancelIntent(),
            }
          : <SingleActivator, Intent>{
              SingleActivator(LogicalKeyboardKey.keyO, control: true):
                  OpenIntent(),
              SingleActivator(LogicalKeyboardKey.keyN, control: true):
                  CreateIntent(),
              SingleActivator(LogicalKeyboardKey.escape): CancelIntent(),
            },
      child: FocusableActionDetector(
        focusNode: this.focusNode,
        autofocus: this.isCreate == null,
        actions: {
          OpenIntent: CallbackAction<OpenIntent>(onInvoke: (_) => open()),
          CreateIntent: CallbackAction<CreateIntent>(onInvoke: (_) => create()),
          CancelIntent: CallbackAction<CancelIntent>(onInvoke: (_) => cancel()),
        },
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            SizedBox(
              width: 420,
              height: 280,
              child: Card(
                child: Padding(
                  padding: const EdgeInsets.all(16.0),
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                    crossAxisAlignment: CrossAxisAlignment.center,
                    children: [
                      ExcludeSemantics(
                        child: Image.asset(
                          'assets/icon.png',
                          width: 128,
                          height: 128,
                          filterQuality: FilterQuality.medium,
                        ),
                      ),
                      isCreate == null
                          ? SelectActionWidget(storeController)
                          : PasswordWidget(
                              passwordController,
                              isCreate!,
                              focusNode2,
                            )
                    ],
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  void create() async {
    final path = await FilePicker.newFile();
    if (path != null) {
      setState(() => isCreate = true);
    }
  }

  void open() async {
    final path = await FilePicker.existingFolder();
    if (path != null) {
      setState(() => isCreate = false);
    }
  }

  void cancel() {
    setState(() {
      this.isCreate = null;
      this.passwordController.clear();
    });
  }
}

class PasswordWidget extends StatelessWidget {
  final TextEditingController passwordController;
  final bool isCreate;
  final FocusNode focusNode;

  const PasswordWidget(
    this.passwordController,
    this.isCreate,
    this.focusNode, {
    Key? key,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisAlignment: MainAxisAlignment.spaceEvenly,
      children: [
        SizedBox(
          width: 368,
          child: Semantics(
            label: 'Password field',
            focusable: true,
            child: TextField(
              controller: this.passwordController,
              enableSuggestions: false,
              autocorrect: false,
              obscureText: true,
              focusNode: this.focusNode,
              autofocus: true,
              decoration: InputDecoration(hintText: 'Password'),
            ),
          ),
        ),
        SizedBox.fromSize(size: Size(0, 16)),
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
          children: [
            SemanticButton(
              label: 'Cancel',
              text: 'Cancel',
              action: () {
                Actions.maybeInvoke(context, CancelIntent());
              },
            ),
            SemanticButton(
              label: isCreate ? 'Create' : 'Open',
              text: isCreate ? 'Create store' : 'Open store',
              action: () => isCreate ? create() : open(),
            ),
          ],
        )
      ],
    );
  }

  void create() {}

  void open() {}
}

class SelectActionWidget extends StatelessWidget {
  final TextEditingController storeController;

  const SelectActionWidget(
    this.storeController, {
    Key? key,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
          children: [
            SemanticButton(
              label: 'Create',
              text: 'Create a new store',
              action: () => invokeCreate(context),
            ),
            SemanticButton(
              label: 'Open',
              text: 'Open an existing store',
              action: () => invokeOpen(context),
            ),
          ],
        ),
      ],
    );
  }

  void invokeCreate(BuildContext context) {
    Actions.maybeInvoke(context, CreateIntent());
  }

  void invokeOpen(BuildContext context) {
    Actions.maybeInvoke(context, OpenIntent());
  }
}

class SemanticButton extends StatelessWidget {
  const SemanticButton({
    Key? key,
    required this.label,
    required this.text,
    required this.action,
  }) : super(key: key);

  final String label;
  final String text;
  final void Function() action;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 180,
      height: 48,
      child: Semantics(
        label: text,
        button: true,
        focusable: true,
        onTap: action,
        child: OutlinedButton(
          onPressed: action,
          child: Text(label),
        ),
      ),
    );
  }
}

class OpenIntent extends Intent {
  const OpenIntent();
}

class CreateIntent extends Intent {
  const CreateIntent();
}

class CancelIntent extends Intent {
  const CancelIntent();
}
