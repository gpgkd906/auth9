from playwright.sync_api import sync_playwright
import sys

def run():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context()
        page = context.new_page()
        
        try:
            # Step 1: Login first
            print('Step 1: Login to dashboard...')
            page.goto('http://localhost:3002/login')
            page.wait_for_selector('input[name="username"]', timeout=10000)
            page.fill('input[name="username"]', 'admin')
            page.fill('input[name="password"]', 'NewSecurePass456!')
            page.click('button[type="submit"]')
            page.wait_for_url('**/dashboard**', timeout=15000)
            
            print('Step 2: On dashboard, looking for logout...')
            
            # Step 2: Click logout
            page.click('text=Logout')
            
            # Wait for redirect
            page.wait_for_timeout(3000)
            
            current_url = page.url
            print(f'After logout URL: {current_url}')
            
            # Check homepage content
            page_content = page.content()
            is_logged_out = 'You are currently not logged in' in page_content
            print(f'Shows logged out message: {is_logged_out}')
            
            # Step 3: Try to access dashboard directly
            print('Step 3: Trying to access dashboard directly...')
            page.goto('http://localhost:3002/dashboard')
            page.wait_for_timeout(1000)
            
            final_url = page.url
            print(f'Direct dashboard access URL: {final_url}')
            
            # Should be redirected back to home or show login
            is_redirected = final_url == 'http://localhost:3002/' or final_url.endswith('/')
            
            print('\n=== Scenario 5 Result ===')
            if is_logged_out and is_redirected:
                print('PASS: Logout works correctly')
            else:
                print(f'FAIL: is_logged_out={is_logged_out}, is_redirected={is_redirected}')
                sys.exit(1)
            
        except Exception as e:
            print('ERROR:', str(e))
            print('\n=== Scenario 5 Result ===')
            print('FAIL:', str(e))
            sys.exit(1)
        finally:
            browser.close()

if __name__ == '__main__':
    run()
