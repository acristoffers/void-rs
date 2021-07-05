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

import 'package:flutter/material.dart';
import 'package:void_gui/widgets/filepicker.dart';

class LoginPage extends StatefulWidget {
  LoginPage({Key? key}) : super(key: key);

  @override
  _LoginPageState createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  final storeController = TextEditingController();
  final passwordController = TextEditingController();
  bool? isCreate;

  @override
  Widget build(BuildContext context) {
    return Column(
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
                      ? SelectActionWidget(
                          storeController,
                          (x) => setState(() => isCreate = x),
                        )
                      : PasswordWidget(
                          passwordController,
                          isCreate!,
                          (x) => setState(() => isCreate = x),
                        )
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class PasswordWidget extends StatelessWidget {
  final TextEditingController passwordController;
  final void Function(bool?) setCreate;
  final bool isCreate;

  const PasswordWidget(
    this.passwordController,
    this.isCreate,
    this.setCreate, {
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
              autofocus: true,
              decoration: InputDecoration(hintText: 'Password'),
            ),
          ),
        ),
        SizedBox.fromSize(size: Size(0, 16)),
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
          children: [
            SizedBox(
              width: 180,
              height: 48,
              child: Semantics(
                label: 'Cancel',
                button: true,
                focusable: true,
                onTap: () {
                  setCreate(null);
                  passwordController.clear();
                },
                child: OutlinedButton(
                  onPressed: () {
                    setCreate(null);
                    passwordController.clear();
                  },
                  child: Text('Cancel'),
                ),
              ),
            ),
            SizedBox(
              width: 180,
              height: 48,
              child: Semantics(
                label: isCreate ? 'Create store' : 'Open store',
                button: true,
                focusable: true,
                onTap: () => isCreate ? create() : open(),
                child: OutlinedButton(
                  onPressed: () => isCreate ? create() : open(),
                  child: Text(isCreate ? 'Create' : 'Open'),
                ),
              ),
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
  final void Function(bool) setCreate;

  const SelectActionWidget(
    this.storeController,
    this.setCreate, {
    Key? key,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceEvenly,
          children: [
            SizedBox(
              width: 180,
              height: 48,
              child: Semantics(
                label: 'Create new store',
                button: true,
                focusable: true,
                onTap: this.createStore,
                child: OutlinedButton(
                  onPressed: this.createStore,
                  child: Text('Create'),
                ),
              ),
            ),
            SizedBox(
              width: 180,
              height: 48,
              child: Semantics(
                label: 'Open existing store',
                button: true,
                focusable: true,
                onTap: this.openStore,
                child: OutlinedButton(
                  onPressed: this.openStore,
                  child: Text('Open'),
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }

  void createStore() async {
    final path = await FilePicker.newFile();
    if (path != null) {
      setCreate(true);
      print(path);
    }
  }

  void openStore() async {
    final path = await FilePicker.existingFolder();
    if (path != null) {
      setCreate(false);
      print(path);
    }
  }
}
