import asyncio
import aiohttp

RESET_TOKEN = "ed77f54774aa64b90a52106539eb79cd8244083d09add93fee58aba2da2f8bd1"

async def reset(session, i):
    async with session.post('http://localhost:8080/api/v1/auth/reset-password',
        json={'token': RESET_TOKEN, 'new_password': f'NewPass{i}!'}) as resp:
        return resp.status

async def main():
    async with aiohttp.ClientSession() as session:
        tasks = [reset(session, i) for i in range(50)]
        results = await asyncio.gather(*tasks)
        success = results.count(200)
        print(f'Success: {success}, Failed: {len(results) - success}')
        assert success <= 1, f'RACE CONDITION: {success} successful resets!'

if __name__ == "__main__":
    asyncio.run(main())