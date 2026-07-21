# Ambiguous loop identity

Two distinct loops begin on the same source line in the same function. Debug
location alone cannot select one loop, so the integrated optimization query
must retain an `identity.ambiguous` decline and must not execute variants.
