# The matching engine

The matching engine is a deterministic state machine that takes an order as
input and updates its state accordingly.

## Orders

* There are buy, sell, and management orders.
* Buy orders are either matched or thrown out.

## Buy orders

A buy order is a tuple `(P, t, d, c)`, where

* `P` is a predicate, describing the desired resource,
* `t` is a time stamp in the future, describing the time at which the buyer
  would like to acquire the resource,
* `d` is a natural number, describing the duration for which the buyer desires
  to acquire the resource as a multiple of the unit time (see below), and
* `c` is the offered unit price.

Thus, `d * c` is the maximum amount transferred when a matching is made.

## Sell orders

A sell order is a tuple `(R, t_s, d_e, G)`, where

* `R` is a description of a resource,
* `t_s` is a point in time when the order becomes valid,
* `d_e` is the duration (as a number of unit durations) after which the order
  expires,
* `G` is a list of pairs `(m, p)`, the so-called offering. The value `m` is a
  multiplier and `p` is a corresponding price.

Let us assume that `(m_1, p_1), (m_2, p_2), ... (m_N, p_N)` is the list of all
pairs. Then it holds that `p_{i+1} > p_i` and `m_{i+1} * p_i > p_{i+1}`. The
price for the given resource for a duration of `m_1 * m_2 * ... * m_i` is `p_i`.

In other words, `m_1` is the smallest duration for which the seller is willing
to sell. The next larger duration that can be bought is `m_2 * m_1` at the price
`p_2`. When matching a buy order to a sell order, different durations can be
combined. The choice of constraints listed above (the duration being multiples
of each other) allows for efficient matching.

## State of the Matching Engine

The state of the matching engine is a tuple `(u, B)`, where

* `u` is the unit time, and
* `B` is the order book that contains the currently valid sell orders.

## Update

Consider a buy order `(P, t, d, c)`. Generally, if a buy order matches a sell
order, it matches against the cheaper of the following combination of durations
of the offering:

* The minimal combination that exactly matches `d`.
* The minimal combination that exceeds `d`.

As a result, depending on the offering, a buyer might acquire a resource for
longer than `d`. However, the amount transacted is always `d*c` at maximum.
