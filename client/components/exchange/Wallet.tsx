import { useBalances } from "@/store/balances";
import { useEffect } from "react";

const Wallet = () => {
    const { items, fetch } = useBalances();

    useEffect(() => {
        fetch();
    }, []);

  return (
    <div>
        <h1>Wallet</h1>
        {items.length > 0 ? (
            items.map((item) => (
                <div key={item.token_id}>
                    <h2>{item.token_id}</h2>
                    <p>{item.available}</p>
                </div>
            ))
        ) : (
            <p>No balances found</p>
        )}
    </div>
  )
}

export default Wallet