from playwright.sync_api import sync_playwright
import sys

def run():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context()
        page = context.new_page()
        
        try:
            # Step 1: Navigate to demo login page
            print('Step 1: Navigate to demo login...')
            page.goto('http://localhost:3002/login')
            page.wait_for_load_state('networkidle')
            
            # Step 2: Should redirect to Keycloak
            print('Step 2: Current URL:', page.url)
            print('Waiting for Keycloak login page...')
            
            page.wait_for_selector('input[name="username"]', timeout=10000)
            
            # Step 3: Fill in credentials
            print('Step 3: Filling credentials...')
            page.fill('input[name="username"]', 'admin')
            page.fill('input[name="password"]', 'NewSecurePass456!')
            page.click('button[type="submit"]')
            
            # Wait for redirect to dashboard
            print('Waiting for dashboard...')
            page.wait_for_url('**/dashboard**', timeout=15000)
            
            print('Step 4: Dashboard URL:', page.url)
            title = page.title()
            print('Page title:', title)
            
            page_content = page.content()
            print('Has Identity Token:', 'Identity Token' in page_content)
            print('Has Dashboard:', 'Dashboard' in page_content)
            print('Has admin@auth9.local:', 'admin@auth9.local' in page_content)
            print('Has Auth9-signed:', 'Auth9-signed' in page_content)
            
            print('\n=== Scenario 3 Result ===')
            print('PASS: Successfully logged in and redirected to dashboard')
            
        except Exception as e:
            print('ERROR:', str(e))
            print('\n=== Scenario 3 Result ===')
            print('FAIL:', str(e))
            sys.exit(1)
        finally:
            browser.close()

if __name__ == '__main__':
    run()
