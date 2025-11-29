/**
 * Stripe.js Mock for E2E Testing
 *
 * This mock replaces the real Stripe.js library to allow testing payment flows
 * without dealing with cross-origin iframe restrictions.
 */

export const stripeMockScript = `
(function() {
	console.log('[Stripe Mock] Initializing mock Stripe.js');

	// Store test card data globally
	window.__stripeTestCard = null;

	// Mock Stripe constructor function
	const MockStripe = function(publishableKey) {
		console.log('[Stripe Mock] Created Stripe instance with key:', publishableKey);

		return {
			elements: function() {
				console.log('[Stripe Mock] Created Elements instance');

				return {
					create: function(type, options) {
						console.log('[Stripe Mock] Created element type:', type);

						if (type === 'card') {
							return {
								mount: function(selector) {
									console.log('[Stripe Mock] Mounted card element');
									// Create a mock input field instead of iframe
									const mountPoint = typeof selector === 'string'
										? document.querySelector(selector)
										: selector;

									if (mountPoint) {
										mountPoint.innerHTML = \`
											<div style="padding: 12px;">
												<input
													id="mock-card-number"
													type="text"
													placeholder="Card number"
													style="width: 100%; padding: 8px; margin-bottom: 8px; background: transparent; border: 1px solid rgba(255,255,255,0.2); color: white; border-radius: 4px;"
												/>
												<div style="display: flex; gap: 8px;">
													<input
														id="mock-card-expiry"
														type="text"
														placeholder="MM/YY"
														style="flex: 1; padding: 8px; background: transparent; border: 1px solid rgba(255,255,255,0.2); color: white; border-radius: 4px;"
													/>
													<input
														id="mock-card-cvc"
														type="text"
														placeholder="CVC"
														style="flex: 1; padding: 8px; background: transparent; border: 1px solid rgba(255,255,255,0.2); color: white; border-radius: 4px;"
													/>
													<input
														id="mock-card-postal"
														type="text"
														placeholder="ZIP"
														style="flex: 1; padding: 8px; background: transparent; border: 1px solid rgba(255,255,255,0.2); color: white; border-radius: 4px;"
													/>
												</div>
											</div>
										\`;

										// Store card data when user types
										mountPoint.addEventListener('input', function(e) {
											window.__stripeTestCard = {
												number: document.getElementById('mock-card-number').value,
												expiry: document.getElementById('mock-card-expiry').value,
												cvc: document.getElementById('mock-card-cvc').value,
												postal: document.getElementById('mock-card-postal').value
											};
										});
									}
								},
								unmount: function() {
									console.log('[Stripe Mock] Unmounted card element');
								},
								on: function(event, handler) {
									console.log('[Stripe Mock] Registered event:', event);
								}
							};
						}

						return {};
					}
				};
			},

			confirmCardPayment: async function(clientSecret, options) {
				console.log('[Stripe Mock] confirmCardPayment called');
				console.log('[Stripe Mock] Client secret:', clientSecret);
				console.log('[Stripe Mock] Card data:', window.__stripeTestCard);

				// Simulate processing delay
				await new Promise(resolve => setTimeout(resolve, 500));

				const cardNumber = window.__stripeTestCard?.number || '';

				// Test card logic based on Stripe's test cards
				if (cardNumber === '4242424242424242') {
					// Success card
					console.log('[Stripe Mock] Payment succeeded with test card');
					return {
						paymentIntent: {
							id: 'pi_test_success',
							status: 'succeeded'
						}
					};
				} else if (cardNumber === '4000000000000002') {
					// Declined card
					console.log('[Stripe Mock] Payment declined with test card');
					return {
						error: {
							code: 'card_declined',
							message: 'Your card was declined.'
						}
					};
				} else if (cardNumber === '4000000000000069') {
					// Expired card
					console.log('[Stripe Mock] Expired card error');
					return {
						error: {
							code: 'expired_card',
							message: 'Your card has expired.'
						}
					};
				} else if (cardNumber === '4000000000000127') {
					// Incorrect CVC
					console.log('[Stripe Mock] Incorrect CVC error');
					return {
						error: {
							code: 'incorrect_cvc',
							message: 'Your card\\'s security code is incorrect.'
						}
					};
				} else {
					// Default to success for any other number (for testing)
					console.log('[Stripe Mock] Default success for card:', cardNumber);
					return {
						paymentIntent: {
							id: 'pi_test_default',
							status: 'succeeded'
						}
					};
				}
			}
		};
	};

	// Assign to window.Stripe
	window.Stripe = MockStripe;

	// Mock loadStripe function (used by @stripe/stripe-js)
	// Make it a named function for better debugging
	const mockLoadStripe = async function(publishableKey, options) {
		console.log('[Stripe Mock] loadStripe called with key:', publishableKey);
		await new Promise(resolve => setTimeout(resolve, 10)); // Simulate async load
		return MockStripe(publishableKey);
	};

	window.loadStripe = mockLoadStripe;

	// Also expose on window.Stripe in case it's called differently
	if (!window.Stripe) {
		window.Stripe = MockStripe;
	}

	console.log('[Stripe Mock] Mock Stripe.js loaded successfully');
	console.log('[Stripe Mock] window.loadStripe available:', typeof window.loadStripe === 'function');
	console.log('[Stripe Mock] window.Stripe available:', typeof window.Stripe === 'function');
})();
`;
