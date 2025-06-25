#!/usr/bin/env python3

import random

class RandomBinaryModel:
    def __init__(self, seed: int | None = None) -> None:
        if seed is not None:
            random.seed(seed)

    def predict(self, X):
        prediction = random.randint(0, 1)
        return [random.randint(0, 1) for _ in range(len(X))]
