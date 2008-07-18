from user import User
from split import Split
from bluechips.model import meta
from bluechips.lib.helpers import round_currency
from decimal import Decimal
import random

class Expenditure(object):
    def __repr__(self):
        return '<Expenditure: spender: %s spent: %s>' % (self.spender,
                                                         self.amount)

    def even_split(self):
        """
        Split up an expenditure evenly among the resident users
        """
        
        residents = meta.Session.query(User).filter(User.resident==True)
        split_percentage = Decimal(100) / Decimal(residents.count())
        self.split(dict((resident, split_percentage) for resident in residents))
    
    def split(self, split_dict):
        """
        Split up an expenditure.
        
        split_dict should be a dict mapping from bluechips.model:User
        objects to a decimal:Decimal object representing the percentage
        that user is responsible for.
        
        Percentages will be normalized to sum to 100%.
        
        If the split leaks or gains money due to rounding errors, the
        pennies will be randomly distributed to one of the users.
        
        I mean, come on. You're already living together. Are you really
        going to squabble over a few pennies?
        """
        
        map(meta.Session.delete, meta.Session.query(Split).\
                filter_by(expenditure_id=self.id))
        
        total = sum(split_dict.itervalues())
        
        for user, share in split_dict.iteritems():
            split_dict[user] = share / total
            
        amounts_dict = dict()
        
        for user, share in split_dict.iteritems():
            amounts_dict[user] = round_currency(split_dict[user] * self.amount)
        
        difference = self.amount - sum(amounts_dict.itervalues())
        
        if difference > 0:
            for i in xrange(difference * 100):
                winner = random.choice(amounts_dict.keys())
                amounts_dict[winner] += Decimal('0.01')
        elif difference < 0:
            for i in xrange(difference * -100):
                winner = random.choice(amounts_dict.keys())
                amounts_dict[winner] -= Decimal('0.01')
        
        for user, share in amounts_dict.iteritems():
            s = Split()
            s.expenditure = self
            s.user = user
            s.share = share
            meta.Session.save(s)

__all__ = ['Expenditure']
