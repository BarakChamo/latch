"""Verify that the example returns a greeting."""

import unittest

from latch_python_example import greeting


class GreetingTest(unittest.TestCase):
    def test_includes_the_name(self) -> None:
        self.assertEqual(greeting("Ada"), "Hello, Ada!")
