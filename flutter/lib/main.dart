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

import 'package:desktop_window/desktop_window.dart';
import 'package:flutter/material.dart';
import 'package:void_gui/widgets/layout.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await DesktopWindow.setMinWindowSize(const Size(800, 600));
  runApp(VoidApp());
}

class VoidApp extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'Void Encrypted Store',
      darkTheme: buildThemeData(),
      home: LayoutPage(),
    );
  }

  ThemeData buildThemeData() {
    return ThemeData(
      brightness: Brightness.dark,
      appBarTheme: const AppBarTheme(backgroundColor: Colors.black),
      scaffoldBackgroundColor: const Color(0xFF121212),
      backgroundColor: const Color(0xFF121212),
      primaryColor: Colors.black,
      accentColor: const Color(0xFF1DB954),
      iconTheme: const IconThemeData().copyWith(color: Colors.white),
      fontFamily: 'Roboto',
      textTheme: TextTheme(
        headline2: const TextStyle(
          color: Colors.white,
          fontSize: 32.0,
          fontWeight: FontWeight.bold,
        ),
        headline4: TextStyle(
          fontSize: 12.0,
          color: Colors.grey[300],
          fontWeight: FontWeight.w500,
          letterSpacing: 2.0,
        ),
        bodyText1: TextStyle(
          color: Colors.grey[300],
          fontSize: 14.0,
          fontWeight: FontWeight.w600,
          letterSpacing: 1.0,
        ),
        bodyText2: TextStyle(
          color: Colors.grey[300],
          letterSpacing: 1.0,
        ),
      ),
    );
  }
}
