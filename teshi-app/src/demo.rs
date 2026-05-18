//! Demo feature files for first-time exploration.

/// Sample Gherkin features that are loaded when the user clicks "Load Demo Data".
pub const DEMO_FEATURES: &[(&str, &str)] = &[
    ("login.feature", "\
Feature: User Login
  As a registered user
  I want to sign into the system
  So that I can access my dashboard

  Background:
    Given I am on the login page

  @happy
  Scenario: Successful login with valid credentials
    When I enter username \"alice\"
    And I enter password \"correct-password\"
    Then I should see the dashboard
    And I should see \"Welcome, alice\"

  @error
  Scenario: Failed login with wrong password
    When I enter username \"alice\"
    And I enter password \"wrong-password\"
    Then I should see \"Invalid credentials\"
    And I should remain on the login page

  @edge
  Scenario Outline: Login with empty fields
    When I enter username \"<user>\"
    And I enter password \"<pass>\"
    Then I should see \"<error>\"

    Examples:
      | user  | pass  | error                  |
      |       |       | Username is required    |
      | alice |       | Password is required    |
      |       | secret | Username is required    |
"),
    ("checkout.feature", "\
Feature: Shopping Cart Checkout
  As a customer
  I want to complete my purchase
  So that I can receive my items

  @happy
  Scenario: Successfully checkout with items in cart
    Given I have items in my cart
    When I proceed to checkout
    And I enter valid shipping information
    And I select \"Standard\" shipping
    And I confirm my order
    Then I should see an order confirmation
    And I should receive a confirmation email

  @error
  Scenario: Checkout with empty cart
    Given my cart is empty
    When I try to proceed to checkout
    Then I should see \"Your cart is empty\"
    And I should be redirected to the shopping page

  Scenario: Apply promo code during checkout
    Given I have items in my cart
    When I proceed to checkout
    And I enter promo code \"SAVE10\"
    Then I should see a 10% discount applied
    And the total should reflect the discount
"),
    ("search.feature", "\
Feature: Product Search
  As a visitor
  I want to search for products
  So that I can find what I need

  Background:
    Given I am on the search page

  @happy
  Scenario: Search for existing product
    When I search for \"wireless headphones\"
    Then I should see at least one result
    And the results should include \"Wireless Bluetooth Headphones\"

  @error
  Scenario: Search for non-existent product
    When I search for \"zzzznonexistentproduct123\"
    Then I should see \"No results found\"
    And I should see suggestions for similar products

  Scenario Outline: Filter search results
    Given I have searched for \"shoes\"
    When I filter by <category>
    Then all results should be in <category>

    Examples:
      | category    |
      | Running     |
      | Casual      |
      | Formal      |
"),
];
